//! Iterative drops for nodes a parser chains in a loop, such as `x::INT::INT`, `v[1][1]`, `v:a:a` or `q |> WHERE a |> WHERE b`.

use crate::expressions::functions::models::JSONCast;
use crate::expressions::functions::models::JSONExtract;
use crate::expressions::functions::models::Operator;
use crate::expressions::functions::models::Xor;
use crate::expressions::operators::models::Alias;
use crate::expressions::operators::models::ArraySlice;
use crate::expressions::operators::models::Between;
use crate::expressions::operators::models::BinaryOp;
use crate::expressions::operators::models::Cast;
use crate::expressions::operators::models::CollationExpr;
use crate::expressions::operators::models::DotAccess;
use crate::expressions::operators::models::In;
use crate::expressions::operators::models::IsNull;
use crate::expressions::operators::models::IsTrueFalse;
use crate::expressions::operators::models::LikeOp;
use crate::expressions::operators::models::MethodCall;
use crate::expressions::operators::models::Subscript;
use crate::expressions::operators::models::UnaryOp;
use crate::expressions::queries::models::MatchRecognize;
use crate::expressions::queries::models::PipeOperator;
use crate::expressions::statements::models::TableSample;
use crate::expressions::tree::models::Expression;
use crate::expressions::typed_functions::models::AtTimeZone;
use crate::expressions::typed_functions::models::BinaryFunc;
use crate::expressions::typed_functions::models::JsonExtractFunc;
use crate::expressions::typed_functions::models::ModelAttribute;
use crate::traversal::types::AstNode;

/// Frees a node's expression descendants with a worklist, so dropping a long chain cannot overflow the stack.
fn drop_descendants(node: &mut impl AstNode) {
    let mut pending: Vec<Expression> = Vec::new();
    node.take_expressions(&mut pending);
    while let Some(mut expression) = pending.pop() {
        expression.take_expressions(&mut pending);
    }
}

macro_rules! drop_iteratively {
    ($($node:ty),* $(,)?) => {
        $(
            impl Drop for $node {
                fn drop(&mut self) {
                    drop_descendants(self);
                }
            }
        )*
    };
}

drop_iteratively!(
    Alias,
    ArraySlice,
    AtTimeZone,
    Between,
    BinaryFunc,
    BinaryOp,
    Cast,
    CollationExpr,
    DotAccess,
    In,
    IsNull,
    IsTrueFalse,
    JSONCast,
    JSONExtract,
    JsonExtractFunc,
    LikeOp,
    MatchRecognize,
    MethodCall,
    ModelAttribute,
    Operator,
    PipeOperator,
    Subscript,
    TableSample,
    UnaryOp,
    Xor,
);
