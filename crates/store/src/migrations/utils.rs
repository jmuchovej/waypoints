use sea_orm_migration::{
    DbErr,
    prelude::{Alias, IndexCreateStatement, IntoIden, TableRef},
    sea_orm::EntityTrait,
};

pub type SeaDBResult<T> = ::std::result::Result<T, DbErr>;

#[allow(dead_code)]
pub fn table_name<E>(entity: E) -> SeaDBResult<TableRef>
where
    E: EntityTrait,
{
    Ok(TableRef::Table(Alias::new(entity.to_string()).into_iden()))
}

#[allow(dead_code)]
pub fn infer_index_name<E>(entity: E, i: IndexCreateStatement) -> SeaDBResult<String>
where
    E: EntityTrait,
{
    let column = i.get_index_spec().get_column_names().join("-");
    index_name(entity, &column)
}

#[allow(dead_code)]
pub fn index_name<E>(entity: E, c: &str) -> SeaDBResult<String>
where
    E: EntityTrait,
{
    let name = format!("idx-{table}-{c}", table = entity.to_string());
    Ok(name)
}
