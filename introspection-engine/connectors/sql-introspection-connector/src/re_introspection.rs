use crate::introspection_helpers::replace_field_names;
use crate::warnings::*;
use crate::PreviewFeature::NamedConstraints;
use crate::SqlFamilyTrait;
use datamodel::{Datamodel, DefaultValue, FieldType, Ignorable, ValueGenerator};
use introspection_connector::{IntrospectionContext, Warning};
use prisma_value::PrismaValue;
use std::cmp::Ordering;
use std::cmp::Ordering::{Equal, Greater, Less};

pub fn enrich(old_data_model: &Datamodel, new_data_model: &mut Datamodel, ctx: &IntrospectionContext) -> Vec<Warning> {
    let mut warnings = vec![];

    //@@map on models
    let mut changed_model_names = vec![];
    {
        for model in new_data_model.models() {
            if let Some(old_model) =
                old_data_model.find_model_db_name(model.database_name.as_ref().unwrap_or(&model.name))
            {
                if new_data_model.find_model(&old_model.name).is_none() {
                    changed_model_names.push((Model::new(&model.name), Model::new(&old_model.name)))
                }
            }
        }

        //change model names
        for changed_model_name in &changed_model_names {
            let model = new_data_model.find_model_mut(&changed_model_name.0.model);
            model.name = changed_model_name.1.model.clone();
            if model.database_name.is_none() {
                model.database_name = Some(changed_model_name.0.model.clone())
            };
        }

        // change relation types
        for changed_model_name in &changed_model_names {
            let fields_to_be_changed = new_data_model.find_relation_fields_for_model(&changed_model_name.0.model);

            for relation_field in fields_to_be_changed {
                let field = new_data_model.find_relation_field_mut(&relation_field.0, &relation_field.1);
                field.relation_info.to = changed_model_name.1.model.clone();
            }
        }
    }

    let mut changed_index_names = vec![];
    let mut changed_primary_key_names = vec![];
    if ctx.preview_features.contains(NamedConstraints) {
        //custom index names
        {
            for model in new_data_model.models() {
                if let Some(old_model) = &old_data_model.find_model(&model.name) {
                    for index in &model.indices {
                        if let Some(old_index) = old_model.indices.iter().find(|old| old.db_name == index.db_name) {
                            if old_index.name.is_some() {
                                let mf = ModelAndIndex::new(&model.name, old_index.db_name.as_ref().unwrap());
                                changed_index_names.push((mf, old_index.name.clone()))
                            }
                        }
                    }
                }
            }

            //change index name
            for changed_index_name in &changed_index_names {
                let index = new_data_model
                    .find_model_mut(&changed_index_name.0.model)
                    .indices
                    .iter_mut()
                    .find(|i| i.db_name == Some(changed_index_name.0.index_db_name.clone()))
                    .unwrap();
                index.name = changed_index_name.1.clone();
            }
        }

        //custom primary key names
        {
            for model in new_data_model.models() {
                if let Some(old_model) = &old_data_model.find_model(&model.name) {
                    if let Some(primary_key) = &model.primary_key {
                        if let Some(old_primary_key) = &old_model.primary_key {
                            if old_primary_key.fields == primary_key.fields
                                && (old_primary_key.db_name == primary_key.db_name || primary_key.db_name.is_none())
                                && old_primary_key.name.is_some()
                            {
                                let mf = Model::new(&model.name);
                                changed_primary_key_names.push((mf, old_primary_key.name.clone()))
                            }
                        }
                    }
                }
            }

            //change primary key names
            for changed_primary_key_name in &changed_primary_key_names {
                let pk = new_data_model
                    .find_model_mut(&changed_primary_key_name.0.model)
                    .primary_key
                    .as_mut();

                if let Some(primary_key) = pk {
                    primary_key.name = changed_primary_key_name.1.clone()
                }
            }
        }
    }

    // @map on fields
    let mut changed_scalar_field_names = vec![];
    {
        for model in new_data_model.models() {
            if let Some(old_model) = &old_data_model.find_model(&model.name) {
                for field in model.scalar_fields() {
                    if let Some(old_field) =
                        old_model.find_scalar_field_db_name(field.database_name.as_ref().unwrap_or(&field.name))
                    {
                        if model.find_scalar_field(&old_field.name).is_none() {
                            let mf = ModelAndField::new(&model.name, &field.name);
                            changed_scalar_field_names.push((mf, old_field.name.clone()))
                        }
                    }
                }
            }
        }

        //change field name
        for changed_field_name in &changed_scalar_field_names {
            let field = new_data_model.find_scalar_field_mut(&changed_field_name.0.model, &changed_field_name.0.field);
            field.name = changed_field_name.1.clone();
            if field.database_name.is_none() {
                field.database_name = Some(changed_field_name.0.field.clone())
            };
        }

        // change usages in @@id, @@index, @@unique and on RelationInfo.fields
        for changed_field_name in &changed_scalar_field_names {
            let model = new_data_model.find_model_mut(&changed_field_name.0.model);

            if let Some(pk) = &mut model.primary_key {
                replace_field_names(&mut pk.fields, &changed_field_name.0.field, &changed_field_name.1);
            }

            for index in &mut model.indices {
                replace_field_names(&mut index.fields, &changed_field_name.0.field, &changed_field_name.1);
            }
            for field in model.relation_fields_mut() {
                replace_field_names(
                    &mut field.relation_info.fields,
                    &changed_field_name.0.field,
                    &changed_field_name.1,
                );
            }
        }

        // change RelationInfo.references
        for changed_field_name in &changed_scalar_field_names {
            let fields_to_be_changed = new_data_model.find_relation_fields_for_model(&changed_field_name.0.model);
            for f in fields_to_be_changed {
                let field = new_data_model.find_relation_field_mut(&f.0, &f.1);
                replace_field_names(
                    &mut field.relation_info.references,
                    &changed_field_name.0.field,
                    &changed_field_name.1,
                );
            }
        }
    }

    //always keep old virtual relationfield names
    let mut changed_relation_field_names = vec![];
    {
        for new_model in new_data_model.models() {
            for new_field in new_model.relation_fields() {
                if let Some(old_model) = old_data_model.find_model(&new_model.name) {
                    for old_field in old_model.relation_fields() {
                        let (_, old_related_field) = &old_data_model.find_related_field_bang(old_field);
                        let is_many_to_many = old_field.is_list() && old_related_field.is_list();
                        let is_self_relation = old_field.relation_info.to == old_related_field.relation_info.to;

                        let (_, related_field) = &new_data_model.find_related_field_bang(new_field);

                        //the relationinfos of both sides need to be compared since the relationinfo of the
                        // non-fk side does not contain enough information to uniquely identify the correct relationfield
                        let relation_info_partial_eq = old_field.relation_info == new_field.relation_info
                            && old_related_field.relation_info == related_field.relation_info;

                        let mf = ModelAndField::new(&new_model.name, &new_field.name);

                        if relation_info_partial_eq
                            && (!is_many_to_many
                                //For many to many the relation infos always look the same, here we have to look at the relation name,
                                //which translates to the join table name. But in case of self relations we cannot correctly infer the old name
                                || (old_field.relation_info.name == new_field.relation_info.name && !is_self_relation))
                        {
                            changed_relation_field_names.push((mf.clone(), old_field.name.clone()));
                        }
                    }
                }
            }
        }

        for changed_relation_field_name in changed_relation_field_names {
            new_data_model
                .find_relation_field_mut(
                    &changed_relation_field_name.0.model,
                    &changed_relation_field_name.0.field,
                )
                .name = changed_relation_field_name.1;
        }
    }

    //keep old virtual relation names on non M:N relations
    // M:N relations cannot be uniquely identified without ignoring the relationname and their relationnames cant
    // be changed without necessitation db changes since RelationName -> Join table name

    let mut changed_relation_names = vec![];
    {
        for model in new_data_model.models() {
            for field in model.relation_fields() {
                if let Some(old_model) = old_data_model.find_model(&model.name) {
                    for old_field in old_model.relation_fields() {
                        let (_, related_field) = &new_data_model.find_related_field_bang(field);
                        let (_, old_related_field) = &old_data_model.find_related_field_bang(old_field);
                        //the relationinfos of both sides need to be compared since the relationinfo of the
                        // non-fk side does not contain enough information to uniquely identify the correct relationfield

                        let relation_info_partial_eq = old_field.relation_info == field.relation_info
                            && old_related_field.relation_info == related_field.relation_info;
                        let many_to_many = old_field.is_list() && old_related_field.is_list();

                        if relation_info_partial_eq && !many_to_many {
                            let mf = ModelAndField::new(&model.name, &field.name);
                            let other_mf = ModelAndField::new(&field.relation_info.to, &related_field.name);
                            changed_relation_names.push((mf, old_field.relation_info.name.clone()));
                            changed_relation_names.push((other_mf, old_field.relation_info.name.clone()))
                        }
                    }
                }
            }
        }

        for changed_relation_name in changed_relation_names {
            new_data_model
                .find_relation_field_mut(&changed_relation_name.0.model, &changed_relation_name.0.field)
                .relation_info
                .name = changed_relation_name.1;
        }
    }

    // @@map on enums
    let mut changed_enum_names = vec![];
    {
        for enm in new_data_model.enums() {
            if let Some(old_enum) = old_data_model.find_enum_db_name(enm.database_name.as_ref().unwrap_or(&enm.name)) {
                if new_data_model.find_enum(&old_enum.name).is_none() {
                    changed_enum_names.push((Enum { enm: enm.name.clone() }, old_enum.name.clone()))
                }
            }
        }
        for changed_enum_name in &changed_enum_names {
            let enm = new_data_model.find_enum_mut(&changed_enum_name.0.enm);
            enm.name = changed_enum_name.1.clone();
            if enm.database_name.is_none() {
                enm.database_name = Some(changed_enum_name.0.enm.clone());
            }
        }

        for changed_enum_name in &changed_enum_names {
            let fields_to_be_changed = new_data_model.find_enum_fields(&changed_enum_name.0.enm);

            for change2 in fields_to_be_changed {
                let field = new_data_model.find_scalar_field_mut(&change2.0, &change2.1);
                field.field_type = FieldType::Enum(changed_enum_name.1.clone());
            }
        }
    }

    // @map on enum values
    let mut changed_enum_values = vec![];
    {
        for enm in new_data_model.enums() {
            if let Some(old_enum) = old_data_model.find_enum(&enm.name) {
                for value in enm.values() {
                    if let Some(old_value) =
                        old_enum.find_value_db_name(value.database_name.as_ref().unwrap_or(&value.name.to_owned()))
                    {
                        if enm.find_value(&old_value.name).is_none() {
                            let ev = EnumAndValue::new(&enm.name, &value.name);
                            changed_enum_values.push((ev, old_value.name.clone()))
                        }
                    }
                }
            }
        }
        for changed_enum_value in &changed_enum_values {
            let enm = new_data_model.find_enum_mut(&changed_enum_value.0.enm);
            let value = enm.find_value_mut(&changed_enum_value.0.value);
            value.name = changed_enum_value.1.clone();
            if value.database_name.is_none() {
                value.database_name = Some(changed_enum_value.0.value.clone());
            }
        }

        for changed_enum_value in &changed_enum_values {
            let fields_to_be_changed = new_data_model.find_enum_fields(&changed_enum_value.0.enm);

            for field in fields_to_be_changed {
                let field = new_data_model.find_scalar_field_mut(&field.0, &field.1);
                if field.default_value
                    == Some(DefaultValue::new_single(PrismaValue::Enum(
                        changed_enum_value.0.value.clone(),
                    )))
                {
                    field.default_value = Some(DefaultValue::new_single(PrismaValue::Enum(
                        changed_enum_value.1.clone(),
                    )));
                }
            }
        }
    }

    //mysql enum names
    let mut changed_mysql_enum_names = vec![];
    {
        if ctx.sql_family().is_mysql() {
            for enm in new_data_model.enums() {
                if let Some((model_name, field_name)) = &new_data_model.find_enum_fields(&enm.name).first() {
                    if let Some(old_model) = old_data_model.find_model(model_name) {
                        if let Some(old_field) = old_model.find_field(field_name) {
                            if let FieldType::Enum(old_enum_name) = old_field.field_type() {
                                let old_enum = old_data_model.find_enum(&old_enum_name).unwrap();
                                if enm.values == old_enum.values
                                    && old_enum_name != enm.name
                                    && !changed_mysql_enum_names
                                        .iter()
                                        .any(|x: &(String, String, ModelAndField)| x.1 == old_enum_name)
                                {
                                    changed_mysql_enum_names.push((
                                        enm.name.clone(),
                                        old_enum.name.clone(),
                                        ModelAndField::new(model_name, field_name),
                                    ))
                                }
                            }
                        }
                    }
                }
            }

            for changed_enum_name in &changed_mysql_enum_names {
                //adjust enum name
                let enm = new_data_model.find_enum_mut(&changed_enum_name.0);
                enm.name = changed_enum_name.1.clone();

                //adjust Fieldtype on field that uses it
                let field =
                    new_data_model.find_scalar_field_mut(&changed_enum_name.2.model, &changed_enum_name.2.field);
                field.field_type = FieldType::Enum(changed_enum_name.1.clone());
            }
        }
    }

    // Prisma Level Only concepts
    // @default(cuid) / @default(uuid) / @updatedAt
    let mut re_introspected_prisma_level_cuids = vec![];
    let mut re_introspected_prisma_level_uuids = vec![];
    let mut re_introspected_updated_at = vec![];
    {
        for model in new_data_model.models() {
            for field in model.scalar_fields() {
                if let Some(old_model) = old_data_model.find_model(&model.name) {
                    if let Some(old_field) = old_model.find_scalar_field(&field.name) {
                        if field.default_value.is_none() && field.field_type.is_string() {
                            if old_field.default_value == Some(DefaultValue::new_expression(ValueGenerator::new_cuid()))
                            {
                                re_introspected_prisma_level_cuids.push(ModelAndField::new(&model.name, &field.name));
                            }

                            if old_field.default_value == Some(DefaultValue::new_expression(ValueGenerator::new_uuid()))
                            {
                                re_introspected_prisma_level_uuids.push(ModelAndField::new(&model.name, &field.name));
                            }
                        }

                        if field.field_type.is_datetime() && old_field.is_updated_at {
                            re_introspected_updated_at.push(ModelAndField::new(&model.name, &field.name));
                        }
                    }
                }
            }
        }

        for cuid in &re_introspected_prisma_level_cuids {
            new_data_model
                .find_scalar_field_mut(&cuid.model, &cuid.field)
                .default_value = Some(DefaultValue::new_expression(ValueGenerator::new_cuid()));
        }

        for uuid in &re_introspected_prisma_level_uuids {
            new_data_model
                .find_scalar_field_mut(&uuid.model, &uuid.field)
                .default_value = Some(DefaultValue::new_expression(ValueGenerator::new_uuid()));
        }

        for updated_at in &re_introspected_updated_at {
            new_data_model
                .find_scalar_field_mut(&updated_at.model, &updated_at.field)
                .is_updated_at = true;
        }
    }

    //@@ignore on models
    //@ignore on fields
    let mut re_introspected_model_ignores = vec![];
    let mut re_introspected_field_ignores = vec![];
    {
        for model in new_data_model.models() {
            if let Some(old_model) = old_data_model.find_model(&model.name) {
                if old_model.is_ignored {
                    re_introspected_model_ignores.push(Model::new(&model.name));
                }

                for field in model.scalar_fields() {
                    if let Some(old_field) = old_model.find_scalar_field(&field.name) {
                        if old_field.is_ignored {
                            re_introspected_field_ignores.push(ModelAndField::new(&model.name, &field.name));
                        }
                    }
                }
            }
        }

        for ignore in &re_introspected_model_ignores {
            new_data_model.find_model_mut(&ignore.model).is_ignored = true;
        }

        for ignore in &re_introspected_field_ignores {
            new_data_model.find_field_mut(&ignore.model, &ignore.field).ignore();
        }
    }

    // comments - we do NOT generate warnings for comments
    {
        let mut re_introspected_model_comments = vec![];
        let mut re_introspected_field_comments = vec![];
        {
            for model in new_data_model.models() {
                for field in &model.fields {
                    if let Some(old_model) = old_data_model.find_model(&model.name) {
                        if old_model.documentation.is_some() {
                            re_introspected_model_comments.push((Model::new(&model.name), &old_model.documentation))
                        }
                        if let Some(old_field) = old_model.find_field(field.name()) {
                            if old_field.documentation().is_some() {
                                re_introspected_field_comments.push((
                                    ModelAndField::new(&model.name, field.name()),
                                    old_field.documentation().map(|s| s.to_string()),
                                ))
                            }
                        }
                    }
                }
            }

            for model_comment in &re_introspected_model_comments {
                new_data_model.find_model_mut(&model_comment.0.model).documentation = model_comment.1.clone();
            }

            for field_comment in &re_introspected_field_comments {
                new_data_model
                    .find_field_mut(&field_comment.0.model, &field_comment.0.field)
                    .set_documentation(field_comment.1.clone());
            }
        }

        let mut re_introspected_enum_comments = vec![];
        let mut re_introspected_enum_value_comments = vec![];
        {
            for enm in new_data_model.enums() {
                for value in &enm.values {
                    if let Some(old_enum) = old_data_model.find_enum(&enm.name) {
                        if old_enum.documentation.is_some() {
                            re_introspected_enum_comments.push((Enum::new(&enm.name), &old_enum.documentation))
                        }
                        if let Some(old_value) = old_enum.find_value(&value.name) {
                            if old_value.documentation.is_some() {
                                re_introspected_enum_value_comments.push((
                                    EnumAndValue::new(&enm.name, &value.name),
                                    old_value.documentation.clone(),
                                ))
                            }
                        }
                    }
                }
            }

            for enum_comment in &re_introspected_enum_comments {
                new_data_model.find_enum_mut(&enum_comment.0.enm).documentation = enum_comment.1.clone();
            }

            for enum_value_comment in &re_introspected_enum_value_comments {
                new_data_model
                    .find_enum_mut(&enum_value_comment.0.enm)
                    .find_value_mut(&enum_value_comment.0.value)
                    .documentation = enum_value_comment.1.clone();
            }
        }
    }

    // restore old model order
    new_data_model.models.sort_by(|model_a, model_b| {
        let model_a_idx = old_data_model.models().position(|model| model.name == model_a.name);
        let model_b_idx = old_data_model.models().position(|model| model.name == model_b.name);

        re_order_putting_new_ones_last(model_a_idx, model_b_idx)
    });

    // restore old enum order
    new_data_model.enums.sort_by(|enum_a, enum_b| {
        let enum_a_idx = old_data_model.enums().position(|enm| enm.name == enum_a.name);
        let enum_b_idx = old_data_model.enums().position(|enm| enm.name == enum_b.name);

        re_order_putting_new_ones_last(enum_a_idx, enum_b_idx)
    });

    //warnings

    if !changed_model_names.is_empty() {
        let models: Vec<_> = changed_model_names.iter().map(|c| c.1.clone()).collect();
        warnings.push(warning_enriched_with_map_on_model(&models));
    }

    if !changed_index_names.is_empty() {
        let index: Vec<_> = changed_index_names.iter().map(|c| c.0.clone()).collect();
        warnings.push(warning_enriched_with_custom_index_names(&index));
    }

    if !changed_primary_key_names.is_empty() {
        let pk: Vec<_> = changed_primary_key_names.iter().map(|c| c.0.clone()).collect();
        warnings.push(warning_enriched_with_custom_primary_key_names(&pk));
    }

    if !changed_scalar_field_names.is_empty() {
        let models_and_fields: Vec<_> = changed_scalar_field_names
            .iter()
            .map(|c| ModelAndField::new(&c.0.model, &c.1))
            .collect();
        warnings.push(warning_enriched_with_map_on_field(&models_and_fields));
    }

    if !changed_enum_names.is_empty() {
        let enums: Vec<_> = changed_enum_names.iter().map(|c| Enum::new(&c.1)).collect();
        warnings.push(warning_enriched_with_map_on_enum(&enums));
    }

    if !changed_enum_values.is_empty() {
        let enums_and_values: Vec<_> = changed_enum_values
            .iter()
            .map(|c| EnumAndValue::new(&c.0.enm, &c.1))
            .collect();

        warnings.push(warning_enriched_with_map_on_enum_value(&enums_and_values));
    }

    if !re_introspected_prisma_level_cuids.is_empty() {
        warnings.push(warning_enriched_with_cuid(&re_introspected_prisma_level_cuids));
    }

    if !re_introspected_prisma_level_uuids.is_empty() {
        warnings.push(warning_enriched_with_uuid(&re_introspected_prisma_level_uuids));
    }

    if !re_introspected_updated_at.is_empty() {
        warnings.push(warning_enriched_with_updated_at(&re_introspected_updated_at));
    }

    if !re_introspected_model_ignores.is_empty() {
        warnings.push(warning_enriched_models_with_ignore(&re_introspected_model_ignores));
    }

    if !re_introspected_field_ignores.is_empty() {
        warnings.push(warning_enriched_fields_with_ignore(&re_introspected_field_ignores));
    }

    warnings
}

fn re_order_putting_new_ones_last(enum_a_idx: Option<usize>, enum_b_idx: Option<usize>) -> Ordering {
    match (enum_a_idx, enum_b_idx) {
        (None, None) => Equal,
        (None, Some(_)) => Greater,
        (Some(_), None) => Less,
        (Some(a_idx), Some(b_idx)) => a_idx.cmp(&b_idx),
    }
}
