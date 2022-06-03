use crate::algebra::op::*;
use crate::context::TempDbContext;
use crate::data::tuple::OwnTuple;
use crate::data::tuple_set::{BindingMap, TableId, TupleSet};
use crate::data::value::StaticValue;
use crate::ddl::reify::TableInfo;
use crate::parser::{Pair, Rule};
use anyhow::Result;
use std::collections::BTreeSet;
use std::fmt::{Debug, Formatter};

#[derive(thiserror::Error, Debug)]
pub(crate) enum AlgebraParseError {
    #[error("{0} cannot be chained")]
    Unchainable(String),

    #[error("wrong argument type for {0}({1}): {2}")]
    WrongArgumentType(String, usize, String),

    #[error("Table not found {0}")]
    TableNotFound(String),

    #[error("Wrong table kind {0:?}")]
    WrongTableKind(TableId),

    #[error("Table id not found {0:?}")]
    TableIdNotFound(TableId),

    #[error("Not enough arguments for {0}")]
    NotEnoughArguments(String),

    #[error("Value error {0:?}")]
    ValueError(StaticValue),

    #[error("Parse error {0}")]
    Parse(String),

    #[error("Data key conflict {0:?}")]
    KeyConflict(OwnTuple),

    #[error("Value not found for {0:?}")]
    ValueNotFound(OwnTuple),

    #[error("No association between {0} and {1}")]
    NoAssociation(String, String),

    #[error("Duplicate binding {0}")]
    DuplicateBinding(String),

    #[error("Aggregate function in forbidden place")]
    AggregateFnNotAllowed,

    #[error("Scalar function in forbidden place")]
    ScalarFnNotAllowed,
}

pub(crate) fn assert_rule(pair: &Pair, rule: Rule, name: &str, u: usize) -> Result<()> {
    if pair.as_rule() == rule {
        Ok(())
    } else {
        Err(AlgebraParseError::WrongArgumentType(
            name.to_string(),
            u,
            format!("{:?}", pair.as_rule()),
        )
        .into())
    }
}

// this looks stupid but is the easiest way to get downcasting
pub(crate) enum RaBox<'a> {
    Insertion(Box<Insertion<'a>>),
    TaggedInsertion(Box<TaggedInsertion<'a>>),
    FromValues(Box<RelationFromValues>),
    TableScan(Box<TableScan<'a>>),
    WhereFilter(Box<WhereFilter<'a>>),
    SelectOp(Box<SelectOp<'a>>),
    AssocOp(Box<AssocOp<'a>>),
    LimitOp(Box<LimitOp<'a>>),
    Cartesian(Box<CartesianJoin<'a>>),
    NestedLoopLeft(Box<NestedLoopLeft<'a>>),
    SortOp(Box<SortOp<'a>>),
    MergeJoin(Box<MergeJoin<'a>>),
    ConcatOp(Box<ConcatOp<'a>>),
    UnionOp(Box<UnionOp<'a>>),
    IntersectOp(Box<IntersectOp<'a>>),
    SymDiffOp(Box<SymDiffOp<'a>>),
    DiffOp(Box<DiffOp<'a>>),
    GroupOp(Box<GroupOp<'a>>),
    DeleteOp(Box<DeleteOp<'a>>),
    UpdateOp(Box<UpdateOp<'a>>),
    WalkOp(Box<WalkOp<'a>>),
}

impl<'a> RaBox<'a> {
    pub(crate) fn sources(&self) -> Vec<&RaBox> {
        match self {
            RaBox::Insertion(inner) => vec![&inner.source],
            RaBox::TaggedInsertion(_inner) => vec![],
            RaBox::FromValues(_inner) => vec![],
            RaBox::WalkOp(_inner) => vec![],
            RaBox::TableScan(_inner) => vec![],
            RaBox::WhereFilter(inner) => vec![&inner.source],
            RaBox::SelectOp(inner) => vec![&inner.source],
            RaBox::AssocOp(inner) => vec![&inner.source],
            RaBox::LimitOp(inner) => vec![&inner.source],
            RaBox::Cartesian(inner) => vec![&inner.left, &inner.right],
            RaBox::NestedLoopLeft(inner) => vec![&inner.left],
            RaBox::SortOp(inner) => vec![&inner.source],
            RaBox::MergeJoin(inner) => vec![&inner.left, &inner.right],
            RaBox::ConcatOp(inner) => inner.sources.iter().collect(),
            RaBox::UnionOp(inner) => inner.sources.iter().collect(),
            RaBox::DiffOp(inner) => inner.sources.iter().collect(),
            RaBox::IntersectOp(inner) => inner.sources.iter().collect(),
            RaBox::SymDiffOp(inner) => vec![&inner.sources[0], &inner.sources[1]],
            RaBox::GroupOp(inner) => vec![&inner.source],
            RaBox::DeleteOp(inner) => vec![&inner.source],
            RaBox::UpdateOp(inner) => vec![&inner.source],
        }
    }
}

impl<'a> Debug for RaBox<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}", self.name())?;
        for sub in self.sources() {
            write!(f, " {:?}", sub)?;
        }
        write!(f, ")")
    }
}

impl<'b> RelationalAlgebra for RaBox<'b> {
    fn name(&self) -> &str {
        match self {
            RaBox::Insertion(inner) => inner.name(),
            RaBox::TaggedInsertion(inner) => inner.name(),
            RaBox::FromValues(inner) => inner.name(),
            RaBox::TableScan(inner) => inner.name(),
            RaBox::WhereFilter(inner) => inner.name(),
            RaBox::SelectOp(inner) => inner.name(),
            RaBox::AssocOp(inner) => inner.name(),
            RaBox::LimitOp(inner) => inner.name(),
            RaBox::Cartesian(inner) => inner.name(),
            RaBox::NestedLoopLeft(inner) => inner.name(),
            RaBox::SortOp(inner) => inner.name(),
            RaBox::MergeJoin(inner) => inner.name(),
            RaBox::ConcatOp(inner) => inner.name(),
            RaBox::UnionOp(inner) => inner.name(),
            RaBox::IntersectOp(inner) => inner.name(),
            RaBox::SymDiffOp(inner) => inner.name(),
            RaBox::DiffOp(inner) => inner.name(),
            RaBox::GroupOp(inner) => inner.name(),
            RaBox::DeleteOp(inner) => inner.name(),
            RaBox::UpdateOp(inner) => inner.name(),
            RaBox::WalkOp(inner) => inner.name(),
        }
    }

    fn bindings(&self) -> Result<BTreeSet<String>> {
        match self {
            RaBox::Insertion(inner) => inner.bindings(),
            RaBox::TaggedInsertion(inner) => inner.bindings(),
            RaBox::FromValues(inner) => inner.bindings(),
            RaBox::TableScan(inner) => inner.bindings(),
            RaBox::WhereFilter(inner) => inner.bindings(),
            RaBox::SelectOp(inner) => inner.bindings(),
            RaBox::AssocOp(inner) => inner.bindings(),
            RaBox::LimitOp(inner) => inner.bindings(),
            RaBox::Cartesian(inner) => inner.bindings(),
            RaBox::NestedLoopLeft(inner) => inner.bindings(),
            RaBox::SortOp(inner) => inner.bindings(),
            RaBox::MergeJoin(inner) => inner.bindings(),
            RaBox::ConcatOp(inner) => inner.bindings(),
            RaBox::UnionOp(inner) => inner.bindings(),
            RaBox::IntersectOp(inner) => inner.bindings(),
            RaBox::SymDiffOp(inner) => inner.bindings(),
            RaBox::DiffOp(inner) => inner.bindings(),
            RaBox::GroupOp(inner) => inner.bindings(),
            RaBox::DeleteOp(inner) => inner.bindings(),
            RaBox::UpdateOp(inner) => inner.bindings(),
            RaBox::WalkOp(inner) => inner.bindings(),
        }
    }

    fn binding_map(&self) -> Result<BindingMap> {
        match self {
            RaBox::Insertion(inner) => inner.binding_map(),
            RaBox::TaggedInsertion(inner) => inner.binding_map(),
            RaBox::FromValues(inner) => inner.binding_map(),
            RaBox::TableScan(inner) => inner.binding_map(),
            RaBox::WhereFilter(inner) => inner.binding_map(),
            RaBox::SelectOp(inner) => inner.binding_map(),
            RaBox::AssocOp(inner) => inner.binding_map(),
            RaBox::LimitOp(inner) => inner.binding_map(),
            RaBox::Cartesian(inner) => inner.binding_map(),
            RaBox::NestedLoopLeft(inner) => inner.binding_map(),
            RaBox::SortOp(inner) => inner.binding_map(),
            RaBox::MergeJoin(inner) => inner.binding_map(),
            RaBox::ConcatOp(inner) => inner.binding_map(),
            RaBox::UnionOp(inner) => inner.binding_map(),
            RaBox::IntersectOp(inner) => inner.binding_map(),
            RaBox::SymDiffOp(inner) => inner.binding_map(),
            RaBox::DiffOp(inner) => inner.binding_map(),
            RaBox::GroupOp(inner) => inner.binding_map(),
            RaBox::DeleteOp(inner) => inner.binding_map(),
            RaBox::UpdateOp(inner) => inner.binding_map(),
            RaBox::WalkOp(inner) => inner.binding_map(),
        }
    }

    fn iter<'a>(&'a self) -> Result<Box<dyn Iterator<Item = Result<TupleSet>> + 'a>> {
        match self {
            RaBox::Insertion(inner) => inner.iter(),
            RaBox::TaggedInsertion(inner) => inner.iter(),
            RaBox::FromValues(inner) => inner.iter(),
            RaBox::TableScan(inner) => inner.iter(),
            RaBox::WhereFilter(inner) => inner.iter(),
            RaBox::SelectOp(inner) => inner.iter(),
            RaBox::AssocOp(inner) => inner.iter(),
            RaBox::LimitOp(inner) => inner.iter(),
            RaBox::Cartesian(inner) => inner.iter(),
            RaBox::NestedLoopLeft(inner) => inner.iter(),
            RaBox::SortOp(inner) => inner.iter(),
            RaBox::MergeJoin(inner) => inner.iter(),
            RaBox::ConcatOp(inner) => inner.iter(),
            RaBox::UnionOp(inner) => inner.iter(),
            RaBox::IntersectOp(inner) => inner.iter(),
            RaBox::SymDiffOp(inner) => inner.iter(),
            RaBox::DiffOp(inner) => inner.iter(),
            RaBox::GroupOp(inner) => inner.iter(),
            RaBox::DeleteOp(inner) => inner.iter(),
            RaBox::UpdateOp(inner) => inner.iter(),
            RaBox::WalkOp(inner) => inner.iter(),
        }
    }

    fn identity(&self) -> Option<TableInfo> {
        match self {
            RaBox::Insertion(inner) => inner.identity(),
            RaBox::TaggedInsertion(inner) => inner.identity(),
            RaBox::FromValues(inner) => inner.identity(),
            RaBox::TableScan(inner) => inner.identity(),
            RaBox::WhereFilter(inner) => inner.identity(),
            RaBox::SelectOp(inner) => inner.identity(),
            RaBox::AssocOp(inner) => inner.identity(),
            RaBox::LimitOp(inner) => inner.identity(),
            RaBox::Cartesian(inner) => inner.identity(),
            RaBox::NestedLoopLeft(inner) => inner.identity(),
            RaBox::SortOp(inner) => inner.identity(),
            RaBox::MergeJoin(inner) => inner.identity(),
            RaBox::ConcatOp(inner) => inner.identity(),
            RaBox::UnionOp(inner) => inner.identity(),
            RaBox::IntersectOp(inner) => inner.identity(),
            RaBox::SymDiffOp(inner) => inner.identity(),
            RaBox::DiffOp(inner) => inner.identity(),
            RaBox::GroupOp(inner) => inner.identity(),
            RaBox::DeleteOp(inner) => inner.identity(),
            RaBox::UpdateOp(inner) => inner.identity(),
            RaBox::WalkOp(inner) => inner.identity(),
        }
    }
}

pub(crate) fn build_relational_expr<'a>(
    ctx: &'a TempDbContext,
    mut pair: Pair,
) -> Result<RaBox<'a>> {
    if pair.as_rule() == Rule::ra_arg {
        pair = pair.into_inner().next().unwrap();
    }
    assert_rule(&pair, Rule::ra_expr, pair.as_str(), 0)?;
    let mut built: Option<RaBox> = None;
    for pair in pair.into_inner() {
        let mut pairs = pair.into_inner();
        let pair = pairs.next().unwrap();
        match pair.as_str() {
            NAME_INSERTION => {
                built = Some(RaBox::Insertion(Box::new(Insertion::build(
                    ctx, built, pairs, false,
                )?)))
            }
            NAME_UPSERT => {
                built = Some(RaBox::Insertion(Box::new(Insertion::build(
                    ctx, built, pairs, true,
                )?)))
            }
            NAME_TAGGED_INSERTION => {
                built = Some(RaBox::TaggedInsertion(Box::new(TaggedInsertion::build(
                    ctx, built, pairs, false,
                )?)))
            }
            NAME_TAGGED_UPSERT => {
                built = Some(RaBox::TaggedInsertion(Box::new(TaggedInsertion::build(
                    ctx, built, pairs, true,
                )?)))
            }
            NAME_RELATION_FROM_VALUES => {
                built = Some(RaBox::FromValues(Box::new(RelationFromValues::build(
                    ctx, built, pairs,
                )?)));
            }
            NAME_FROM => {
                built = Some(build_from_clause(ctx, built, pairs)?);
            }
            NAME_WHERE => {
                built = Some(RaBox::WhereFilter(Box::new(WhereFilter::build(
                    ctx, built, pairs,
                )?)))
            }
            NAME_SELECT => {
                built = Some(RaBox::SelectOp(Box::new(SelectOp::build(
                    ctx, built, pairs,
                )?)))
            }
            n @ (NAME_TAKE | NAME_SKIP) => {
                built = Some(RaBox::LimitOp(Box::new(LimitOp::build(
                    ctx, built, pairs, n,
                )?)))
            }
            NAME_SORT => built = Some(RaBox::SortOp(Box::new(SortOp::build(ctx, built, pairs)?))),
            n @ (NAME_INNER_JOIN | NAME_LEFT_JOIN | NAME_RIGHT_JOIN | NAME_OUTER_JOIN) => {
                built = Some(RaBox::MergeJoin(Box::new(MergeJoin::build(
                    ctx, built, pairs, n,
                )?)))
            }
            NAME_CONCAT => {
                built = Some(RaBox::ConcatOp(Box::new(ConcatOp::build(
                    ctx, built, pairs,
                )?)))
            }
            NAME_UNION => {
                built = Some(RaBox::UnionOp(Box::new(UnionOp::build(ctx, built, pairs)?)))
            }
            NAME_INTERSECT => {
                built = Some(RaBox::IntersectOp(Box::new(IntersectOp::build(
                    ctx, built, pairs,
                )?)))
            }
            NAME_DIFF => built = Some(RaBox::DiffOp(Box::new(DiffOp::build(ctx, built, pairs)?))),
            NAME_SYM_DIFF => {
                built = Some(RaBox::SymDiffOp(Box::new(SymDiffOp::build(
                    ctx, built, pairs,
                )?)))
            }
            NAME_GROUP => {
                built = Some(RaBox::GroupOp(Box::new(GroupOp::build(ctx, built, pairs)?)))
            }
            NAME_DELETE => {
                built = Some(RaBox::DeleteOp(Box::new(DeleteOp::build(
                    ctx, built, pairs,
                )?)))
            }
            NAME_UPDATE => {
                built = Some(RaBox::UpdateOp(Box::new(UpdateOp::build(
                    ctx, built, pairs,
                )?)))
            }
            NAME_WALK => built = Some(RaBox::WalkOp(Box::new(WalkOp::build(ctx, built, pairs)?))),
            name => {
                unimplemented!("{}", name)
            }
        }
    }
    Ok(built.unwrap())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::data::tuple::Tuple;
    use crate::parser::{CozoParser, Rule};
    use crate::runtime::options::default_read_options;
    use crate::runtime::session::tests::create_test_db;
    use anyhow::Result;
    use pest::Parser;
    use std::collections::BTreeMap;
    use std::time::Instant;

    const HR_DATA: &str = include_str!("../../test_data/hr.json");

    #[test]
    fn parse_ra() -> Result<()> {
        let (_db, mut sess) = create_test_db("_test_parser.db");
        let start = Instant::now();
        {
            let ctx = sess.temp_ctx(true);
            let s = r#"
                           Values(v: [id, name], [[100, 'confidential'], [101, 'top secret']])
                          .Upsert(:Department, d: {...v})
                          "#;
            let ra = build_relational_expr(
                &ctx,
                CozoParser::parse(Rule::ra_expr_all, s)
                    .unwrap()
                    .into_iter()
                    .next()
                    .unwrap(),
            )?;
            dbg!(&ra);
            dbg!(ra.get_values()?);
            ctx.txn.commit().unwrap();
        }
        {
            let ctx = sess.temp_ctx(true);
            let s = format!("UpsertTagged({})", HR_DATA);
            let ra = build_relational_expr(
                &ctx,
                CozoParser::parse(Rule::ra_expr_all, &s)
                    .unwrap()
                    .into_iter()
                    .next()
                    .unwrap(),
            )?;
            // for t in ra.iter().unwrap() {
            //     dbg!(t.unwrap());
            // }
            dbg!(&ra);
            dbg!(ra.get_values()?);

            ctx.txn.commit().unwrap();
        }
        let duration_insert = start.elapsed();
        let start = Instant::now();
        {
            let ctx = sess.temp_ctx(true);
            let s = r#"
             From(e:Employee, hj:HasJob, j:Job)
            .Where(e.id >= 122, e.id < 130, e.id == hj._src_id, hj._dst_id == j.id)
            .Select({...e, title: j.title, salary: hj.salary})
            .Skip(1)
            .Take(1)
            "#;
            let ra = build_relational_expr(
                &ctx,
                CozoParser::parse(Rule::ra_expr_all, s)
                    .unwrap()
                    .into_iter()
                    .next()
                    .unwrap(),
            )?;
            dbg!(&ra);
            dbg!(ra.get_values()?);
        }
        let duration_scan = start.elapsed();
        let start = Instant::now();
        {
            let ctx = sess.temp_ctx(true);
            let s = r#"
             From(e:Employee-[:Manages]?->s:Employee)
            .Select(o: {boss: e.first_name ++ ' ' ++ e.last_name, slave: (s.first_name ++ ' ' ++ s.last_name) ~ 'NO~ONE'})
            .Sort(o.boss => desc, o.slave)
            "#;
            let ra = build_relational_expr(
                &ctx,
                CozoParser::parse(Rule::ra_expr_all, s)
                    .unwrap()
                    .into_iter()
                    .next()
                    .unwrap(),
            )?;
            dbg!(&ra);
            dbg!(ra.get_values()?);
        }
        let duration_join = start.elapsed();
        let start = Instant::now();
        {
            let ctx = sess.temp_ctx(true);
            let s = r#"
             From(d:Department<-[id:InDepartment]-e:Employee)
            .Where(e.id >= 122, e.id < 130)
            .Select({...e, dept_name: d.name})
            "#;
            let ra = build_relational_expr(
                &ctx,
                CozoParser::parse(Rule::ra_expr_all, s)
                    .unwrap()
                    .into_iter()
                    .next()
                    .unwrap(),
            )?;
            dbg!(&ra);
            dbg!(ra.get_values()?);
        }
        let duration_join_back = start.elapsed();
        let start = Instant::now();
        {
            let ctx = sess.temp_ctx(true);
            let s = r#"
            OuterJoin(
              From(l:Job),
              From(r:Job),
              l.min_salary == r.max_salary
            ).Select({
                l_id: l.id,
                r_id: r.id,
                l_min: l.min_salary,
                // l_max: l.max_salary,
                // r_min: r.min_salary,
                r_max: r.max_salary,
                l_title: l.title,
                r_title: r.title,
            })
            "#;
            let ra = build_relational_expr(
                &ctx,
                CozoParser::parse(Rule::ra_expr_all, s)
                    .unwrap()
                    .into_iter()
                    .next()
                    .unwrap(),
            )?;
            dbg!(&ra);
            dbg!(ra.get_values()?);
        }
        let duration_outer_join = start.elapsed();
        let start = Instant::now();
        {
            let ctx = sess.temp_ctx(true);
            let s = r#"
             Concat(From(d:Department), From(d:Job))
            "#;
            let ra = build_relational_expr(
                &ctx,
                CozoParser::parse(Rule::ra_expr_all, s)
                    .unwrap()
                    .into_iter()
                    .next()
                    .unwrap(),
            )?;
            dbg!(&ra);
            dbg!(ra.get_values()?);
        }
        let duration_concat = start.elapsed();
        let start = Instant::now();
        {
            let ctx = sess.temp_ctx(true);
            let s = r#"
              From(e:Employee)
             .Select({id: e.id, sum_id: count_with[e.id], nn: count_non_null[e.id.lag[;3]], count: count[null], prev_id: lag[e.id; 1], pprev_id: e.id.lag[;2]})
            "#;
            let ra = build_relational_expr(
                &ctx,
                CozoParser::parse(Rule::ra_expr_all, s)
                    .unwrap()
                    .into_iter()
                    .next()
                    .unwrap(),
            )?;
            dbg!(&ra);
            dbg!(ra.get_values()?);
        }
        let duration_aggr = start.elapsed();

        let start = Instant::now();
        {
            let ctx = sess.temp_ctx(true);
            let s = r#"
              From(e:Employee)
             .Group({*key: e.id % 3, ct: count_with[e.id]})
            "#;
            let ra = build_relational_expr(
                &ctx,
                CozoParser::parse(Rule::ra_expr_all, s)
                    .unwrap()
                    .into_iter()
                    .next()
                    .unwrap(),
            )?;
            dbg!(&ra);
            dbg!(ra.get_values()?);
        }
        let duration_group = start.elapsed();
        let start = Instant::now();
        {
            let ctx = sess.temp_ctx(true);
            let s = r#"
             Diff(From(d:Job).Where(d.id <= 15), From(d:Job).Where(d.id >= 15))
            "#;
            let ra = build_relational_expr(
                &ctx,
                CozoParser::parse(Rule::ra_expr_all, s)
                    .unwrap()
                    .into_iter()
                    .next()
                    .unwrap(),
            )?;
            dbg!(&ra);
            dbg!(ra.get_values()?);
        }
        let duration_union = start.elapsed();

        let start = Instant::now();
        {
            let ctx = sess.temp_ctx(true);
            let s = r#"
              From(e:Employee)
             .Where(e.id >= 110)
             .Delete()
            "#;
            let ra = build_relational_expr(
                &ctx,
                CozoParser::parse(Rule::ra_expr_all, s)
                    .unwrap()
                    .into_iter()
                    .next()
                    .unwrap(),
            )?;
            dbg!(&ra);
            dbg!(ra.get_values()?);

            let s = r#"
               From(e:Employee)
              .Where(e.id > 105)
              .Update(e: {last_name: 'FUCKER: ' ++ e.last_name})
            "#;
            let ra = build_relational_expr(
                &ctx,
                CozoParser::parse(Rule::ra_expr_all, s)
                    .unwrap()
                    .into_iter()
                    .next()
                    .unwrap(),
            )?;
            dbg!(&ra);
            dbg!(ra.get_values()?);

            let s = r#"
                From(e:Employee)
               .Where(count[] < 3)
            "#;
            let ra = build_relational_expr(
                &ctx,
                CozoParser::parse(Rule::ra_expr_all, s)
                    .unwrap()
                    .into_iter()
                    .next()
                    .unwrap(),
            )?;
            dbg!(&ra);
            dbg!(ra.get_values()?);
        }
        let duration_delete = start.elapsed();

        let start = Instant::now();
        {
            let ctx = sess.temp_ctx(true);
            let s = r#"
             Walk(j:Job<-[:HasJob]-e:Employee-[:InDepartment]->d:Department,
                  j => Sort(d.id => asc).Take(10).Where(j.id <= 6, j.id > 3),
                  e => Sort(d.id => asc).Skip(1),
                  j: {
                    id_1_job: j.id,
                    id_2_emp: e.id,
                    id_3_dep: d.id,
                  })
            "#;
            let ra = build_relational_expr(
                &ctx,
                CozoParser::parse(Rule::ra_expr_all, s)
                    .unwrap()
                    .into_iter()
                    .next()
                    .unwrap(),
            )?;
            dbg!(&ra);
            dbg!(ra.get_values()?);
        }
        let duration_walk = start.elapsed();

        let start = Instant::now();
        let mut r_opts = default_read_options();
        r_opts.set_total_order_seek(true);
        r_opts.set_prefix_same_as_start(false);
        let it = sess.main.iterator(&r_opts);
        it.to_first();
        let mut n: BTreeMap<u32, usize> = BTreeMap::new();
        while it.is_valid() {
            let (k, v) = it.pair().unwrap();
            let k = Tuple::new(k);
            let v = Tuple::new(v);
            if v.get_prefix() == 0 {
                *n.entry(k.get_prefix()).or_default() += 1;
            }
            it.next();
        }
        let duration_list = start.elapsed();
        dbg!(
            duration_insert,
            duration_scan,
            duration_join,
            duration_join_back,
            duration_concat,
            duration_outer_join,
            duration_list,
            duration_aggr,
            duration_group,
            duration_union,
            duration_delete,
            duration_walk,
            n
        );
        Ok(())
    }
}