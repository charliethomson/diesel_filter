#[macro_use]
extern crate diesel_filter;
#[macro_use]
extern crate diesel;

use diesel::{prelude::*, PgConnection, RunQueryDsl};

use crate::schema::thingies;

mod schema;

#[derive(DieselFilter, Queryable, Debug)]
#[diesel(table_name = thingies)]
struct Thingy {
    pub id: i32,
    #[filter(insensitive, substring)]
    pub name: Option<String>,
    #[filter(multiple)]
    pub category: Option<String>,
    pub other: Option<String>,
}

fn main() {
    let mut conn = todo!();

    let mut filters = ThingyFilters {
        name: Some("cou".into()),
        category: None,
    };

    let results = Thingy::filter(&filters).load::<Thingy>(&mut conn).unwrap();
    let results2 = Thingy::filtered(&filters, &mut conn).unwrap();

    println!("{:?}", filters);
    println!("{:?}", results);
    println!("{:?}", results2);
}
