use crate::common::*;
use indoc::indoc;

#[test]
fn enum_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url = "file:test.db"
        }

        model Todo {
          id     Int    @id
          status Status
        }

        enum Status {
          DONE
          NOT_DONE
        }
    "#};

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: You defined the enum `Status`. But the current connector does not support enums.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m[1;91menum Status {[0m
        [1;94m12 | [0m  DONE
        [1;94m13 | [0m  NOT_DONE
        [1;94m14 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn scalar_list_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url = "file:test.db"
        }

        generator js {
          provider = "prisma-client-js"
          previewFeatures = ["microsoftSqlServer"]
        }

        model Todo {
          id     Int    @id
          val    String[]
        }
    "#};

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mField "val" in model "Todo" can't be a list. The current connector does not support lists of primitive types.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id     Int    @id
        [1;94m13 | [0m  [1;91mval    String[][0m
        [1;94m14 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn unique_index_names_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url = "sqlserver://"
        }

        generator js {
          provider = "prisma-client-js"
          previewFeatures = ["microsoftSqlServer"]
        }

        model User {
          id         Int @id
          neighborId Int

          @@index([id], name: "metaId")
        }

        model Post {
          id Int @id
          optionId Int

          @@index([id], name: "metaId")
        }
    "#};

    assert!(datamodel::parse_schema(dml).is_ok());
}

#[test]
fn json_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url = "sqlserver://"
        }

        generator js {
          provider = "prisma-client-js"
          previewFeatures = ["microsoftSqlServer"]
        }

        model User {
          id   Int @id
          data Json
        }
    "#};

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating field `data` in model `User`: Field `data` in model `User` can't be of type Json. The current connector does not support the Json type.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id   Int @id
        [1;94m13 | [0m  [1;91mdata Json[0m
        [1;94m14 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn non_unique_relation_criteria_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url = "sqlserver://"
        }

        generator js {
          provider = "prisma-client-js"
          previewFeatures = ["microsoftSqlServer"]
        }

        model Todo {
          id           Int    @id
          assigneeName String
          assignee     User   @relation(fields: [assigneeName], references: [name])
        }

        model User {
          id   Int    @id
          name String
          todos Todo[]
        }
    "#};

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: The argument `references` must refer to a unique criteria in the related model `User`. But it is referencing the following fields that are not a unique criteria: name[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  assigneeName String
        [1;94m14 | [0m  [1;91massignee     User   @relation(fields: [assigneeName], references: [name])[0m
        [1;94m15 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn auto_increment_on_non_primary_column_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url = "sqlserver://"
        }

        generator js {
          provider = "prisma-client-js"
          previewFeatures = ["microsoftSqlServer"]
        }

        model Todo {
          id           Int    @id
          non_primary  Int    @default(autoincrement()) @unique
        }
    "#};

    assert!(datamodel::parse_schema(&dml).is_ok());
}

#[test]
fn key_order_enforcement_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url = "sqlserver://"
        }

        generator js {
          provider = "prisma-client-js"
          previewFeatures = ["microsoftSqlServer"]
        }

        model  Todo {
          id1  Int
          id2  Int
          cats Cat[]

          @@id([id1, id2])
        }

        model Cat {
          id    Int @id
          todo1 Int
          todo2 Int

          rel Todo @relation(fields: [todo1, todo2], references: [id2, id1])
        }
    "#};

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: The argument `references` must refer to a unique criteria in the related model `Todo` using the same order of fields. Please check the ordering in the following fields: `id2, id1`.[0m
          [1;94m-->[0m  [4mschema.prisma:24[0m
        [1;94m   | [0m
        [1;94m23 | [0m
        [1;94m24 | [0m  [1;91mrel Todo @relation(fields: [todo1, todo2], references: [id2, id1])[0m
        [1;94m25 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}
