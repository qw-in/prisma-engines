use crate::common::*;
use indoc::indoc;

#[test]
fn enum_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "postgres"
          url = "postgres://"
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

    assert!(datamodel::parse_schema(dml).is_ok());
}

#[test]
fn scalar_list_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "postgres"
          url = "postgres://"
        }

        model Todo {
          id     Int    @id
          val    String[]
        }
    "#};

    assert!(datamodel::parse_schema(dml).is_ok());
}

#[test]
fn unique_index_names_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "postgres"
          url = "postgres://"
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

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe index name `metaId` is declared multiple times. With the current connector index names have to be globally unique.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m
        [1;94m17 | [0m  @@[1;91mindex([id], name: "metaId")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn json_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "postgres"
          url = "postgres://"
        }

        model User {
          id   Int @id
          data Json
        }
    "#};

    assert!(datamodel::parse_schema(dml).is_ok());
}

#[test]
fn non_unique_relation_criteria_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "sqlite"
          url = "file:test.db"
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
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  assigneeName String
        [1;94m 9 | [0m  [1;91massignee     User   @relation(fields: [assigneeName], references: [name])[0m
        [1;94m10 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error);
}

#[test]
fn auto_increment_on_non_primary_column_support() {
    let dml = indoc! {r#"
        datasource db {
          provider = "postgres"
          url = "postgres://"
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
          provider = "postgres"
          url = "postgres://"
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

    assert!(datamodel::parse_schema(&dml).is_ok());
}
