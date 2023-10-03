use std::{
    collections::{BTreeMap, HashMap},
    marker::PhantomData,
};

use tokio_postgres::{types::ToSql, Row};

use crate::PostgresConn;

pub trait FromSql {
    fn from_postgres(row: &Row) -> Self;
}

pub trait TableType {
    fn table_name() -> &'static str;
    fn cols() -> &'static [&'static str];
}

pub enum WhereType {
    Equals,
    NotEquals,
    In,
    LessThan,
    GreaterThan,
}
impl WhereType {
    fn as_expression(&self, column: &'static str, arg: usize) -> String {
        match *self {
            WhereType::Equals => format!("{} = ${}", column, arg),
            WhereType::NotEquals => format!("{} != ${}", column, arg),
            WhereType::In => format!("{} = any(${})", column, arg),
            WhereType::LessThan => format!("{} < ${}", column, arg),
            WhereType::GreaterThan => format!("{} > ${}", column, arg),
        }
    }
}

pub struct WhereClauseData {
    pub data: Box<dyn ToSql + Sync>,
    pub comparison: WhereType,
}
impl WhereClauseData {
    pub fn new<T: ToSql + Sync + 'static>(data: T, comparison: WhereType) -> WhereClauseData {
        WhereClauseData {
            data: Box::new(data),
            comparison,
        }
    }
}

pub struct WhereClause<T: ToSql + Sync> {
    pub d: WhereClauseData,
    _marker: PhantomData<T>,
}
impl<T: ToSql + Sync + 'static> WhereClause<T> {
    pub fn into_data(self) -> WhereClauseData {
        self.d
    }

    pub fn eq(data: T) -> WhereClause<T> {
        WhereClause {
            d: WhereClauseData {
                data: Box::new(data),
                comparison: WhereType::Equals,
            },
            _marker: PhantomData,
        }
    }
    pub fn neq(data: T) -> WhereClause<T> {
        WhereClause {
            d: WhereClauseData {
                data: Box::new(data),
                comparison: WhereType::NotEquals,
            },
            _marker: PhantomData,
        }
    }
    pub fn in_vec(data: Vec<T>) -> WhereClause<T> {
        WhereClause {
            d: WhereClauseData {
                data: Box::new(data),
                comparison: WhereType::In,
            },
            _marker: PhantomData,
        }
    }
    pub fn lt(data: T) -> WhereClause<T> {
        WhereClause {
            d: WhereClauseData {
                data: Box::new(data),
                comparison: WhereType::LessThan,
            },
            _marker: PhantomData,
        }
    }
    pub fn gt(data: T) -> WhereClause<T> {
        WhereClause {
            d: WhereClauseData {
                data: Box::new(data),
                comparison: WhereType::GreaterThan,
            },
            _marker: PhantomData,
        }
    }
}

#[derive(Default)]
pub struct SelectQuery {
    where_data: HashMap<&'static str, WhereClauseData>,
    ignore_keys: Vec<&'static str>,
    clauses: BTreeMap<SelectClause, String>,
    joins: Vec<String>,
}
impl SelectQuery {
    pub fn new() -> SelectQuery {
        SelectQuery::default()
    }

    pub fn ignore(mut self, column: &'static str) -> Self {
        self.ignore_keys.push(column);
        self
    }
    pub fn limit(mut self, count: i64) -> Self {
        self.clauses
            .insert(SelectClause::Limit, format!("LIMIT {}", count));
        self
    }
    pub fn order_by(mut self, column: &'static str, asc: bool) -> Self {
        self.clauses.insert(
            SelectClause::OrderBy,
            format!("ORDER BY {} {}", column, if asc { "ASC" } else { "DESC" }),
        );
        self
    }

    pub fn condition<T: QueryObject>(mut self, data: T) -> Self {
        self.where_data = data.into_params();
        self
    }

    pub fn inner_join<X: TableType, T: ColumnName, U: ColumnName>(
        &mut self,
        local_key: T,
        foreign_key: U,
    ) {
        self.joins.push(format!(
            "INNER JOIN {} ON {}.{} = MAIN.{}",
            X::table_name(),
            X::table_name(),
            foreign_key.to_string(),
            local_key.to_string()
        ))
    }

    async fn query<T: TableType>(self, db: &PostgresConn, explain: bool) -> Option<Vec<Row>> {
        let mut query = format!(
            "{}SELECT {} FROM {} as MAIN {}",
            if !explain { "" } else { "EXPLAIN "},
            if self.ignore_keys.is_empty() {
                T::cols().join(",")
            } else {
                T::cols()
                    .iter()
                    .filter(|x| !self.ignore_keys.contains(x)).copied()
                    .collect::<Vec<_>>()
                    .join(",")
            },
            T::table_name(),
            self.joins.join(", ")
        );
        let mut variables: Vec<&(dyn ToSql + Sync)> = Vec::new();
        let (where_query, mut where_vars) = self.where_data.to_where(0);
        query += &where_query;
        variables.append(&mut where_vars);
        if !self.clauses.is_empty() {
            query += " "
        };
        query += &self.clauses.into_values().collect::<Vec<_>>().join(" ");
        db.query(&query, variables.as_slice()).await.ok()
    }

    pub async fn query_all<T: TableType + FromSql>(self, db: &PostgresConn) -> Option<Vec<T>> {
        Some(
            self.query::<T>(db, false)
                .await?
                .iter()
                .map(|x| T::from_postgres(x))
                .collect(),
        )
    }

    pub async fn explain<T: TableType + FromSql>(self, db: &PostgresConn) -> Option<Vec<String>> {
        Some(
            self.query::<T>(db, true)
                .await?
                .iter()
                .map(|x| x.get(0))
                .collect::<Vec<String>>()
        )
    }

    pub async fn query_one<T: TableType + FromSql>(self, db: &PostgresConn) -> Option<T> {
        self.query::<T>(db, false)
            .await?
            .iter()
            .map(|x| T::from_postgres(x))
            .next()
    }
}

pub struct UpdateQuery<T: UpdatableObject> {
    where_data: HashMap<&'static str, WhereClauseData>,
    update: T,
}
impl<T: UpdatableObject + TableType + FromSql> UpdateQuery<T> {
    pub fn from(obj: T) -> UpdateQuery<T> {
        UpdateQuery {
            where_data: HashMap::new(),
            update: obj,
        }
    }

    pub fn condition<S: QueryObject>(mut self, data: S) -> Self {
        self.where_data = data.into_params();
        self
    }

    async fn query(self, db: &PostgresConn) -> Option<Vec<Row>> {
        let (keys, mut values) = self.update.as_params();
        let mut query = format!(
            "UPDATE {} SET {}",
            T::table_name(),
            (0..keys.len())
                .map(|x| format!("{} = ${}", keys[x], x + 1))
                .collect::<Vec<String>>()
                .join(",")
        );
        let mut variables: Vec<&(dyn ToSql + Sync)> = Vec::new();
        variables.append(&mut values);
        let (where_query, mut where_vars) = self.where_data.to_where(variables.len());
        query += &where_query;
        variables.append(&mut where_vars);
        query += " RETURNING *";
        db.query(&query, variables.as_slice()).await.ok()
    }

    pub async fn query_all(self, db: &PostgresConn) -> Option<Vec<T>> {
        Some(
            self.query(db)
                .await?
                .iter()
                .map(|x| T::from_postgres(x))
                .collect(),
        )
    }

    pub async fn query_one(self, db: &PostgresConn) -> Option<T> {
        self.query(db)
            .await?
            .iter()
            .map(|x| T::from_postgres(x))
            .next()
    }
}

#[derive(Default)]
pub struct DeleteQuery {
    where_data: HashMap<&'static str, WhereClauseData>,
}
impl DeleteQuery {
    pub fn new() -> DeleteQuery {
        DeleteQuery::default()
    }

    pub fn condition<S: QueryObject>(mut self, data: S) -> Self {
        self.where_data = data.into_params();
        self
    }

    pub async fn query<T: TableType>(self, db: &PostgresConn) -> Option<()> {
        let mut query = format!("DELETE FROM {}", T::table_name());
        let (where_query, where_vars) = self.where_data.to_where(0);
        query += &where_query;
        query += " RETURNING *";
        db.query(&query, where_vars.as_slice())
            .await
            .ok()
            .map(|_| ())
    }
}

pub trait ColumnName {
    fn to_string(self) -> String;
}

#[derive(PartialEq, Eq, Ord, PartialOrd)]
enum SelectClause {
    Limit = 1,
    OrderBy = 0,
}

trait ToWhereClause {
    fn to_where(&self, arg_offset: usize) -> (String, Vec<&(dyn ToSql + Sync)>);
}
impl ToWhereClause for HashMap<&'static str, WhereClauseData> {
    fn to_where(&self, arg_offset: usize) -> (String, Vec<&(dyn ToSql + Sync)>) {
        let mut query = String::new();
        let mut variables = Vec::new();
        if !self.is_empty() {
            query += " WHERE "
        }
        let mut arg_idx: usize = 1 + arg_offset;
        for (column, data) in self.iter() {
            query.push_str(&data.comparison.as_expression(column, arg_idx));
            if arg_idx != self.len() {
                query.push_str(" AND ")
            }
            variables.push(data.data.as_ref());
            arg_idx += 1;
        }
        (query, variables)
    }
}

pub trait QueryObject {
    fn into_params(self) -> HashMap<&'static str, WhereClauseData>;
}
pub trait UpdatableObject {
    fn as_params(&self) -> (&[&str], Vec<&(dyn ToSql + Sync)>);
}
