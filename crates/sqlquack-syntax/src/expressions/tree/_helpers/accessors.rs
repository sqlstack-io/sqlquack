//! Expressions: accessors.

use crate::expressions::names::models::BooleanLiteral;
use crate::expressions::names::models::Column;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::Literal;
use crate::expressions::names::models::Null;
use crate::expressions::names::models::Star;
use crate::expressions::names::models::TableRef;
use crate::expressions::operators::models::Alias;
use crate::expressions::queries::models::Select;
use crate::expressions::tree::main::_type_assertion_result::type_assertion_result;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;

impl Expression {
    /// Source semantics still needing dialect-aware lowering; kept through serialization.
    pub fn source_dialect(&self) -> Option<crate::tokens::models::DialectType> {
        match self {
            Self::Function(f) => f.source_dialect,
            Self::AggregateFunction(f) => f.source_dialect,
            Self::ListAgg(f) => f.source_dialect,
            Self::Select(s) => s.source_dialect,
            Self::CreateTable(t) => t.source_dialect,
            Self::Upsert(u) => u.source_dialect,
            Self::Call(c) => c.source_dialect,
            Self::Hierarchy(h) => h.source_dialect,
            Self::ViewParameter(p) => p.source_dialect,
            Self::StorageProperty(p) => p.source_dialect,
            Self::Hint(h) => h.source_dialect,
            Self::JSONValue(j) => j.source_dialect,
            Self::JSONTable(j) => j.source_dialect,
            Self::JSONColumnDef(j) => j.source_dialect,
            Self::JsonQuery(j) | Self::JsonExtract(j) | Self::JsonExtractScalar(j) => {
                j.source_dialect
            }
            Self::RegexpLike(r) => r.options.as_ref().and_then(|o| o.source_dialect),
            Self::RegexpReplace(r) => r.options.as_ref().and_then(|o| o.source_dialect),
            Self::RegexpExtract(r) => r.options.as_ref().and_then(|o| o.source_dialect),
            Self::RegexpInstr(r) => r.options.as_ref().and_then(|o| o.source_dialect),
            Self::RegexpCount(r) => r.options.as_ref().and_then(|o| o.source_dialect),
            Self::Cube(g) => g.options.as_ref().and_then(|o| o.source_dialect),
            Self::Rollup(g) => g.options.as_ref().and_then(|o| o.source_dialect),
            Self::GroupingSets(g) => g.options.as_ref().and_then(|o| o.source_dialect),
            Self::PartitionByProperty(p) => p.specification.as_ref().and_then(|s| s.source_dialect),

            _ => None,
        }
    }

    /// Create a `Column` variant, boxing the value automatically.
    #[inline]
    pub fn boxed_column(col: Column) -> Self {
        Expression::Column(Box::new(col))
    }

    /// Create a `Table` variant, boxing the value automatically.
    #[inline]
    pub fn boxed_table(t: TableRef) -> Self {
        Expression::Table(Box::new(t))
    }

    /// Whether this is a valid top-level statement, not a bare identifier, literal or call.
    pub fn is_statement(&self) -> bool {
        match self {
            Expression::Select(_)
            | Expression::Union(_)
            | Expression::Intersect(_)
            | Expression::Except(_)
            | Expression::Subquery(_)
            | Expression::Values(_)
            | Expression::PipeOperator(_)
            | Expression::Upsert(_)
            | Expression::Call(_)
            | Expression::Insert(_)
            | Expression::Update(_)
            | Expression::Delete(_)
            | Expression::Copy(_)
            | Expression::Put(_)
            | Expression::Merge(_)
            | Expression::TryCatch(_)
            | Expression::CreateTable(_)
            | Expression::DropTable(_)
            | Expression::Undrop(_)
            | Expression::AlterTable(_)
            | Expression::SplitTable(_)
            | Expression::FlashbackTable(_)
            | Expression::CreateIndex(_)
            | Expression::DropIndex(_)
            | Expression::CreateView(_)
            | Expression::DropView(_)
            | Expression::AlterView(_)
            | Expression::AlterIndex(_)
            | Expression::Truncate(_)
            | Expression::TruncateTable(_)
            | Expression::CreateSchema(_)
            | Expression::DropSchema(_)
            | Expression::DropNamespace(_)
            | Expression::CreateDatabase(_)
            | Expression::DropDatabase(_)
            | Expression::CreateFunction(_)
            | Expression::DropFunction(_)
            | Expression::CreateProcedure(_)
            | Expression::DropProcedure(_)
            | Expression::CreateSequence(_)
            | Expression::CreateSynonym(_)
            | Expression::DropSequence(_)
            | Expression::AlterSequence(_)
            | Expression::CreateTrigger(_)
            | Expression::DropTrigger(_)
            | Expression::CreateType(_)
            | Expression::DropType(_)
            | Expression::Comment(_)
            | Expression::Use(_)
            | Expression::Set(_)
            | Expression::SetStatement(_)
            | Expression::Transaction(_)
            | Expression::Commit(_)
            | Expression::Rollback(_)
            | Expression::Grant(_)
            | Expression::Revoke(_)
            | Expression::Cache(_)
            | Expression::Uncache(_)
            | Expression::LoadData(_)
            | Expression::Pragma(_)
            | Expression::Describe(_)
            | Expression::Show(_)
            | Expression::Kill(_)
            | Expression::Prepare(_)
            | Expression::Execute(_)
            | Expression::Declare(_)
            | Expression::Refresh(_)
            | Expression::AlterSession(_)
            | Expression::LockingStatement(_)
            | Expression::Analyze(_)
            | Expression::AnalyzeStatistics(_)
            | Expression::AnalyzeHistogram(_)
            | Expression::AnalyzeSample(_)
            | Expression::AnalyzeListChainedRows(_)
            | Expression::AnalyzeDelete(_)
            | Expression::Attach(_)
            | Expression::Detach(_)
            | Expression::Install(_)
            | Expression::Summarize(_)
            | Expression::Pivot(_)
            | Expression::Unpivot(_)
            | Expression::Command(_)
            | Expression::ScriptingBlock(_)
            | Expression::Raw(_)
            | Expression::CreateTask(_)
            | Expression::ReturnStmt(_) => true,

            Expression::Annotated(a) => a.this.is_statement(),

            Expression::Alias(a) => a.this.is_statement(),

            _ => false,
        }
    }

    /// Create a literal number expression from an integer.
    pub fn number(n: i64) -> Self {
        Expression::Literal(Box::new(Literal::Number(n.to_string())))
    }

    /// Create a single-quoted literal string expression.
    pub fn string(s: impl Into<String>) -> Self {
        Expression::Literal(Box::new(Literal::String(s.into())))
    }

    /// Create a literal number expression from a float.
    pub fn float(f: f64) -> Self {
        Expression::Literal(Box::new(Literal::Number(f.to_string())))
    }

    /// The inferred type: stored for value expressions, computed for literals, else `None`.
    pub fn inferred_type(&self) -> Option<&DataType> {
        match self {
            Expression::And(op)
            | Expression::Or(op)
            | Expression::Add(op)
            | Expression::Sub(op)
            | Expression::Mul(op)
            | Expression::Div(op)
            | Expression::Mod(op)
            | Expression::Eq(op)
            | Expression::Neq(op)
            | Expression::Lt(op)
            | Expression::Lte(op)
            | Expression::Gt(op)
            | Expression::Gte(op)
            | Expression::Concat(op)
            | Expression::BitwiseAnd(op)
            | Expression::BitwiseOr(op)
            | Expression::BitwiseXor(op)
            | Expression::Adjacent(op)
            | Expression::TsMatch(op)
            | Expression::PropertyEQ(op)
            | Expression::ArrayContainsAll(op)
            | Expression::ArrayContainedBy(op)
            | Expression::ArrayOverlaps(op)
            | Expression::JSONBContainsAllTopKeys(op)
            | Expression::JSONBContainsAnyTopKeys(op)
            | Expression::JSONBDeleteAtPath(op)
            | Expression::ExtendsLeft(op)
            | Expression::ExtendsRight(op)
            | Expression::Is(op)
            | Expression::MemberOf(op)
            | Expression::Match(op)
            | Expression::NullSafeEq(op)
            | Expression::NullSafeNeq(op)
            | Expression::Glob(op)
            | Expression::BitwiseLeftShift(op)
            | Expression::BitwiseRightShift(op) => op.inferred_type.as_ref(),

            Expression::Not(op) | Expression::Neg(op) | Expression::BitwiseNot(op) => {
                op.inferred_type.as_ref()
            }

            Expression::Like(op) | Expression::ILike(op) => op.inferred_type.as_ref(),

            Expression::Cast(c) | Expression::TryCast(c) | Expression::SafeCast(c) => {
                c.inferred_type.as_ref()
            }

            Expression::Column(c) => c.inferred_type.as_ref(),
            Expression::Dot(dot) => dot.inferred_type.as_ref(),
            Expression::Function(f) => f.inferred_type.as_ref(),
            Expression::AggregateFunction(f) => f.inferred_type.as_ref(),
            Expression::WindowFunction(f) => f.inferred_type.as_ref(),
            Expression::Case(c) => c.inferred_type.as_ref(),
            Expression::Array(a) => a.inferred_type.as_ref(),
            Expression::ArrayFunc(a) => a.inferred_type.as_ref(),
            Expression::Paren(p) => p.this.inferred_type(),
            Expression::Annotated(a) => match &a.type_assertion {
                Some(asserted) => Some(type_assertion_result(a.this.inferred_type(), asserted)),
                None => a.this.inferred_type(),
            },
            Expression::Subquery(s) => s.inferred_type.as_ref(),
            Expression::Alias(a) => a.inferred_type.as_ref(),
            Expression::Unnest(u) => u.inferred_type.as_ref(),
            Expression::IfFunc(f) => f.inferred_type.as_ref(),
            Expression::Nvl2(f) => f.inferred_type.as_ref(),
            Expression::Count(f) => f.inferred_type.as_ref(),
            Expression::GroupConcat(f) => f.inferred_type.as_ref(),
            Expression::StringAgg(f) => f.inferred_type.as_ref(),
            Expression::ListAgg(f) => f.inferred_type.as_ref(),
            Expression::SumIf(f) => f.inferred_type.as_ref(),
            Expression::Trim(f) => f.inferred_type.as_ref(),

            Expression::Upper(f)
            | Expression::Lower(f)
            | Expression::Length(f)
            | Expression::LTrim(f)
            | Expression::RTrim(f)
            | Expression::Reverse(f)
            | Expression::Abs(f)
            | Expression::Sqrt(f)
            | Expression::Cbrt(f)
            | Expression::Ln(f)
            | Expression::Exp(f)
            | Expression::Sign(f)
            | Expression::Date(f)
            | Expression::Time(f)
            | Expression::Initcap(f)
            | Expression::Ascii(f)
            | Expression::Chr(f)
            | Expression::Soundex(f)
            | Expression::ByteLength(f)
            | Expression::Hex(f)
            | Expression::LowerHex(f)
            | Expression::Unicode(f)
            | Expression::Typeof(f)
            | Expression::Explode(f)
            | Expression::ExplodeOuter(f)
            | Expression::MapFromEntries(f)
            | Expression::MapKeys(f)
            | Expression::MapValues(f)
            | Expression::ArrayLength(f)
            | Expression::ArraySize(f)
            | Expression::Cardinality(f)
            | Expression::ArrayReverse(f)
            | Expression::ArrayDistinct(f)
            | Expression::ArrayFlatten(f)
            | Expression::ArrayCompact(f)
            | Expression::ToArray(f)
            | Expression::JsonArrayLength(f)
            | Expression::JsonKeys(f)
            | Expression::JsonType(f)
            | Expression::ParseJson(f)
            | Expression::ToJson(f)
            | Expression::Radians(f)
            | Expression::Degrees(f)
            | Expression::Sin(f)
            | Expression::Cos(f)
            | Expression::Tan(f)
            | Expression::Asin(f)
            | Expression::Acos(f)
            | Expression::Atan(f)
            | Expression::IsNan(f)
            | Expression::IsInf(f)
            | Expression::Year(f)
            | Expression::Month(f)
            | Expression::Day(f)
            | Expression::Hour(f)
            | Expression::Minute(f)
            | Expression::Second(f)
            | Expression::DayOfWeek(f)
            | Expression::DayOfWeekIso(f)
            | Expression::DayOfMonth(f)
            | Expression::DayOfYear(f)
            | Expression::WeekOfYear(f)
            | Expression::Quarter(f)
            | Expression::Epoch(f)
            | Expression::EpochMs(f)
            | Expression::BitwiseCount(f)
            | Expression::DateFromUnixDate(f)
            | Expression::UnixDate(f)
            | Expression::UnixSeconds(f)
            | Expression::UnixMillis(f)
            | Expression::UnixMicros(f)
            | Expression::TimeStrToDate(f)
            | Expression::DateToDi(f)
            | Expression::DiToDate(f)
            | Expression::TsOrDiToDi(f)
            | Expression::TsOrDsToDatetime(f)
            | Expression::TsOrDsToTimestamp(f)
            | Expression::YearOfWeek(f)
            | Expression::YearOfWeekIso(f)
            | Expression::SHA(f)
            | Expression::SHA1Digest(f)
            | Expression::TimeToUnix(f)
            | Expression::TimeStrToUnix(f) => f.inferred_type.as_ref(),

            Expression::Power(f)
            | Expression::NullIf(f)
            | Expression::IfNull(f)
            | Expression::Nvl(f)
            | Expression::Contains(f)
            | Expression::StartsWith(f)
            | Expression::EndsWith(f)
            | Expression::Levenshtein(f)
            | Expression::ModFunc(f)
            | Expression::IntDiv(f)
            | Expression::Atan2(f)
            | Expression::AddMonths(f)
            | Expression::MonthsBetween(f)
            | Expression::NextDay(f)
            | Expression::UnixToTimeStr(f)
            | Expression::ArrayContains(f)
            | Expression::ArrayPosition(f)
            | Expression::ArrayAppend(f)
            | Expression::ArrayPrepend(f)
            | Expression::ArrayUnion(f)
            | Expression::ArrayExcept(f)
            | Expression::ArrayRemove(f)
            | Expression::StarMap(f)
            | Expression::MapFromArrays(f)
            | Expression::MapContainsKey(f)
            | Expression::ElementAt(f)
            | Expression::JsonMergePatch(f) => f.inferred_type.as_ref(),

            Expression::Coalesce(f)
            | Expression::Greatest(f)
            | Expression::Least(f)
            | Expression::ArrayConcat(f)
            | Expression::ArrayIntersect(f)
            | Expression::ArrayZip(f)
            | Expression::MapConcat(f)
            | Expression::JsonArray(f) => f.inferred_type.as_ref(),

            Expression::Sum(f)
            | Expression::Avg(f)
            | Expression::Min(f)
            | Expression::Max(f)
            | Expression::ArrayAgg(f)
            | Expression::CountIf(f)
            | Expression::Stddev(f)
            | Expression::StddevPop(f)
            | Expression::StddevSamp(f)
            | Expression::Variance(f)
            | Expression::VarPop(f)
            | Expression::VarSamp(f)
            | Expression::Median(f)
            | Expression::Mode(f)
            | Expression::First(f)
            | Expression::Last(f)
            | Expression::AnyValue(f)
            | Expression::ApproxDistinct(f)
            | Expression::ApproxCountDistinct(f)
            | Expression::LogicalAnd(f)
            | Expression::LogicalOr(f)
            | Expression::Skewness(f)
            | Expression::ArrayConcatAgg(f)
            | Expression::ArrayUniqueAgg(f)
            | Expression::BoolXorAgg(f)
            | Expression::BitwiseAndAgg(f)
            | Expression::BitwiseOrAgg(f)
            | Expression::BitwiseXorAgg(f) => f.inferred_type.as_ref(),

            _ => None,
        }
    }

    /// Sets the inferred type on value-producing expressions; a no-op for the others.
    pub fn set_inferred_type(&mut self, dt: DataType) {
        match self {
            Expression::And(op)
            | Expression::Or(op)
            | Expression::Add(op)
            | Expression::Sub(op)
            | Expression::Mul(op)
            | Expression::Div(op)
            | Expression::Mod(op)
            | Expression::Eq(op)
            | Expression::Neq(op)
            | Expression::Lt(op)
            | Expression::Lte(op)
            | Expression::Gt(op)
            | Expression::Gte(op)
            | Expression::Concat(op)
            | Expression::BitwiseAnd(op)
            | Expression::BitwiseOr(op)
            | Expression::BitwiseXor(op)
            | Expression::Adjacent(op)
            | Expression::TsMatch(op)
            | Expression::PropertyEQ(op)
            | Expression::ArrayContainsAll(op)
            | Expression::ArrayContainedBy(op)
            | Expression::ArrayOverlaps(op)
            | Expression::JSONBContainsAllTopKeys(op)
            | Expression::JSONBContainsAnyTopKeys(op)
            | Expression::JSONBDeleteAtPath(op)
            | Expression::ExtendsLeft(op)
            | Expression::ExtendsRight(op)
            | Expression::Is(op)
            | Expression::MemberOf(op)
            | Expression::Match(op)
            | Expression::NullSafeEq(op)
            | Expression::NullSafeNeq(op)
            | Expression::Glob(op)
            | Expression::BitwiseLeftShift(op)
            | Expression::BitwiseRightShift(op) => op.inferred_type = Some(dt),

            Expression::Not(op) | Expression::Neg(op) | Expression::BitwiseNot(op) => {
                op.inferred_type = Some(dt)
            }

            Expression::Like(op) | Expression::ILike(op) => op.inferred_type = Some(dt),

            Expression::Cast(c) | Expression::TryCast(c) | Expression::SafeCast(c) => {
                c.inferred_type = Some(dt)
            }

            Expression::Column(c) => c.inferred_type = Some(dt),
            Expression::Dot(dot) => dot.inferred_type = Some(dt),
            Expression::Function(f) => f.inferred_type = Some(dt),
            Expression::AggregateFunction(f) => f.inferred_type = Some(dt),
            Expression::WindowFunction(f) => f.inferred_type = Some(dt),
            Expression::Case(c) => c.inferred_type = Some(dt),
            Expression::Array(a) => a.inferred_type = Some(dt),
            Expression::ArrayFunc(a) => a.inferred_type = Some(dt),
            Expression::Paren(p) => p.this.set_inferred_type(dt),
            Expression::Annotated(a) => a.this.set_inferred_type(dt),
            Expression::Subquery(s) => s.inferred_type = Some(dt),
            Expression::Alias(a) => a.inferred_type = Some(dt),
            Expression::Unnest(u) => u.inferred_type = Some(dt),
            Expression::IfFunc(f) => f.inferred_type = Some(dt),
            Expression::Nvl2(f) => f.inferred_type = Some(dt),
            Expression::Count(f) => f.inferred_type = Some(dt),
            Expression::GroupConcat(f) => f.inferred_type = Some(dt),
            Expression::StringAgg(f) => f.inferred_type = Some(dt),
            Expression::ListAgg(f) => f.inferred_type = Some(dt),
            Expression::SumIf(f) => f.inferred_type = Some(dt),
            Expression::Trim(f) => f.inferred_type = Some(dt),

            Expression::Upper(f)
            | Expression::Lower(f)
            | Expression::Length(f)
            | Expression::LTrim(f)
            | Expression::RTrim(f)
            | Expression::Reverse(f)
            | Expression::Abs(f)
            | Expression::Sqrt(f)
            | Expression::Cbrt(f)
            | Expression::Ln(f)
            | Expression::Exp(f)
            | Expression::Sign(f)
            | Expression::Date(f)
            | Expression::Time(f)
            | Expression::Initcap(f)
            | Expression::Ascii(f)
            | Expression::Chr(f)
            | Expression::Soundex(f)
            | Expression::ByteLength(f)
            | Expression::Hex(f)
            | Expression::LowerHex(f)
            | Expression::Unicode(f)
            | Expression::Typeof(f)
            | Expression::Explode(f)
            | Expression::ExplodeOuter(f)
            | Expression::MapFromEntries(f)
            | Expression::MapKeys(f)
            | Expression::MapValues(f)
            | Expression::ArrayLength(f)
            | Expression::ArraySize(f)
            | Expression::Cardinality(f)
            | Expression::ArrayReverse(f)
            | Expression::ArrayDistinct(f)
            | Expression::ArrayFlatten(f)
            | Expression::ArrayCompact(f)
            | Expression::ToArray(f)
            | Expression::JsonArrayLength(f)
            | Expression::JsonKeys(f)
            | Expression::JsonType(f)
            | Expression::ParseJson(f)
            | Expression::ToJson(f)
            | Expression::Radians(f)
            | Expression::Degrees(f)
            | Expression::Sin(f)
            | Expression::Cos(f)
            | Expression::Tan(f)
            | Expression::Asin(f)
            | Expression::Acos(f)
            | Expression::Atan(f)
            | Expression::IsNan(f)
            | Expression::IsInf(f)
            | Expression::Year(f)
            | Expression::Month(f)
            | Expression::Day(f)
            | Expression::Hour(f)
            | Expression::Minute(f)
            | Expression::Second(f)
            | Expression::DayOfWeek(f)
            | Expression::DayOfWeekIso(f)
            | Expression::DayOfMonth(f)
            | Expression::DayOfYear(f)
            | Expression::WeekOfYear(f)
            | Expression::Quarter(f)
            | Expression::Epoch(f)
            | Expression::EpochMs(f)
            | Expression::BitwiseCount(f)
            | Expression::DateFromUnixDate(f)
            | Expression::UnixDate(f)
            | Expression::UnixSeconds(f)
            | Expression::UnixMillis(f)
            | Expression::UnixMicros(f)
            | Expression::TimeStrToDate(f)
            | Expression::DateToDi(f)
            | Expression::DiToDate(f)
            | Expression::TsOrDiToDi(f)
            | Expression::TsOrDsToDatetime(f)
            | Expression::TsOrDsToTimestamp(f)
            | Expression::YearOfWeek(f)
            | Expression::YearOfWeekIso(f)
            | Expression::SHA(f)
            | Expression::SHA1Digest(f)
            | Expression::TimeToUnix(f)
            | Expression::TimeStrToUnix(f) => f.inferred_type = Some(dt),

            Expression::Power(f)
            | Expression::NullIf(f)
            | Expression::IfNull(f)
            | Expression::Nvl(f)
            | Expression::Contains(f)
            | Expression::StartsWith(f)
            | Expression::EndsWith(f)
            | Expression::Levenshtein(f)
            | Expression::ModFunc(f)
            | Expression::IntDiv(f)
            | Expression::Atan2(f)
            | Expression::AddMonths(f)
            | Expression::MonthsBetween(f)
            | Expression::NextDay(f)
            | Expression::UnixToTimeStr(f)
            | Expression::ArrayContains(f)
            | Expression::ArrayPosition(f)
            | Expression::ArrayAppend(f)
            | Expression::ArrayPrepend(f)
            | Expression::ArrayUnion(f)
            | Expression::ArrayExcept(f)
            | Expression::ArrayRemove(f)
            | Expression::StarMap(f)
            | Expression::MapFromArrays(f)
            | Expression::MapContainsKey(f)
            | Expression::ElementAt(f)
            | Expression::JsonMergePatch(f) => f.inferred_type = Some(dt),

            Expression::Coalesce(f)
            | Expression::Greatest(f)
            | Expression::Least(f)
            | Expression::ArrayConcat(f)
            | Expression::ArrayIntersect(f)
            | Expression::ArrayZip(f)
            | Expression::MapConcat(f)
            | Expression::JsonArray(f) => f.inferred_type = Some(dt),

            Expression::Sum(f)
            | Expression::Avg(f)
            | Expression::Min(f)
            | Expression::Max(f)
            | Expression::ArrayAgg(f)
            | Expression::CountIf(f)
            | Expression::Stddev(f)
            | Expression::StddevPop(f)
            | Expression::StddevSamp(f)
            | Expression::Variance(f)
            | Expression::VarPop(f)
            | Expression::VarSamp(f)
            | Expression::Median(f)
            | Expression::Mode(f)
            | Expression::First(f)
            | Expression::Last(f)
            | Expression::AnyValue(f)
            | Expression::ApproxDistinct(f)
            | Expression::ApproxCountDistinct(f)
            | Expression::LogicalAnd(f)
            | Expression::LogicalOr(f)
            | Expression::Skewness(f)
            | Expression::ArrayConcatAgg(f)
            | Expression::ArrayUniqueAgg(f)
            | Expression::BoolXorAgg(f)
            | Expression::BitwiseAndAgg(f)
            | Expression::BitwiseOrAgg(f)
            | Expression::BitwiseXorAgg(f) => f.inferred_type = Some(dt),

            _ => {}
        }
    }

    /// Create an unqualified column reference (e.g. `name`).
    pub fn column(name: impl Into<String>) -> Self {
        Expression::Column(Box::new(Column {
            name: Identifier::new(name),
            table: None,
            join_mark: false,
            trailing_comments: Vec::new(),
            span: None,
            inferred_type: None,
        }))
    }

    /// Create a qualified column reference (`table.column`).
    pub fn qualified_column(table: impl Into<String>, column: impl Into<String>) -> Self {
        Expression::Column(Box::new(Column {
            name: Identifier::new(column),
            table: Some(Identifier::new(table)),
            join_mark: false,
            trailing_comments: Vec::new(),
            span: None,
            inferred_type: None,
        }))
    }

    /// Create a bare identifier expression (not a column reference).
    pub fn identifier(name: impl Into<String>) -> Self {
        Expression::Identifier(Identifier::new(name))
    }

    /// Create a NULL expression
    pub fn null() -> Self {
        Expression::Null(Null)
    }

    /// Create a TRUE expression
    pub fn true_() -> Self {
        Expression::Boolean(BooleanLiteral { value: true })
    }

    /// Create a FALSE expression
    pub fn false_() -> Self {
        Expression::Boolean(BooleanLiteral { value: false })
    }

    /// Create a wildcard star (`*`) expression with no EXCEPT/REPLACE/RENAME modifiers.
    pub fn star() -> Self {
        Expression::Star(Star {
            table: None,
            except: None,
            replace: None,
            rename: None,
            trailing_comments: Vec::new(),
            span: None,
        })
    }

    /// Wrap this expression in an `AS` alias (e.g. `expr AS name`).
    pub fn alias(self, name: impl Into<String>) -> Self {
        Expression::Alias(Box::new(Alias::new(self, Identifier::new(name))))
    }

    /// Check if this is a SELECT expression
    pub fn is_select(&self) -> bool {
        matches!(self, Expression::Select(_))
    }

    /// Try to get as a Select
    pub fn as_select(&self) -> Option<&Select> {
        match self {
            Expression::Select(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get as a mutable Select
    pub fn as_select_mut(&mut self) -> Option<&mut Select> {
        match self {
            Expression::Select(s) => Some(s),
            _ => None,
        }
    }
}
