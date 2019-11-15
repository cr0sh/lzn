use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;
use std::io::Write;
#[derive(AsExpression, FromSqlRow, Debug, Clone, Copy, PartialEq, Eq)]
#[sql_type = "Text"]
pub enum Provider {
    Lezhin,
    Naver,
}

impl ToSql<Text, Sqlite> for Provider {
	fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
		let value = match self {
			Self::Lezhin => "lezhin",
			Self::Naver => "naver",
		};
		<String as ToSql<Text, Sqlite>>::to_sql(&value.to_string(), out)
	}
}

impl FromSql<Text, Sqlite> for Provider {
	fn from_sql(bytes: Option<&<Sqlite as Backend>::RawValue>) -> deserialize::Result<Self> {
		match unsafe{<*const str as FromSql<Text, Sqlite>>::from_sql(bytes)?.as_ref()}.unwrap() {
			"lezhin" => Ok(Self::Lezhin),
            "naver" => Ok(Self::Naver),
			_ => Err("Unrecognized enum variant".into()),
		}
	}
}
