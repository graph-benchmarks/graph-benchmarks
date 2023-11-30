use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::benchmarks)]
pub struct Benchmark {
    pub id: i32,
    pub nodes: i32,
}
