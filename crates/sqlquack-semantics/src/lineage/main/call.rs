//! `call`: the function a node calls, by its dispatch name, and its arguments.

use sqlquack_syntax::Expression;

use crate::lineage::_helpers::checks::signatures::dispatch_name;
use crate::lineage::_helpers::checks::signatures::named;

/// The built-in call `node` makes: its name and arguments, when signatures know its form.
pub(crate) fn call(node: &Expression) -> Option<(String, Vec<&Expression>)> {
    let arguments: Vec<&Expression> = match node {
        Expression::Function(function) => {
            return Some((
                dispatch_name(&function.name),
                function.args.iter().collect(),
            ));
        }
        Expression::AggregateFunction(function) => {
            return Some((
                dispatch_name(&function.name),
                function.args.iter().collect(),
            ));
        }
        Expression::Sum(function)
        | Expression::Avg(function)
        | Expression::Min(function)
        | Expression::Max(function)
        | Expression::Stddev(function)
        | Expression::StddevPop(function)
        | Expression::StddevSamp(function)
        | Expression::Variance(function)
        | Expression::VarPop(function)
        | Expression::VarSamp(function) => vec![&function.this],
        Expression::Lag(function) | Expression::Lead(function) => {
            let mut arguments: Vec<&Expression> = vec![&function.this];
            arguments.extend(function.offset.iter());
            arguments.extend(function.default.iter());
            arguments
        }
        Expression::NTile(function) => {
            return named("ntile", function.num_buckets.iter().collect());
        }
        Expression::NullIf(function) => {
            return named("nullif", vec![&function.this, &function.expression]);
        }
        Expression::IfNull(function) => {
            return named("ifnull", vec![&function.this, &function.expression]);
        }
        Expression::FirstValue(function) | Expression::LastValue(function) => vec![&function.this],
        Expression::Coalesce(function)
        | Expression::Greatest(function)
        | Expression::Least(function) => function.expressions.iter().collect(),
        Expression::StringAgg(function) => {
            let mut arguments: Vec<&Expression> = vec![&function.this];
            arguments.extend(function.separator.iter());
            arguments
        }
        Expression::ListAgg(function) => {
            let mut arguments: Vec<&Expression> = vec![&function.this];
            arguments.extend(function.separator.iter());
            return named("listagg", arguments);
        }
        Expression::Upper(function)
        | Expression::Lower(function)
        | Expression::LTrim(function)
        | Expression::RTrim(function)
        | Expression::Abs(function)
        | Expression::Sqrt(function)
        | Expression::Length(function) => vec![&function.this],
        Expression::Substring(function) => {
            let mut arguments: Vec<&Expression> = vec![&function.this, &function.start];
            arguments.extend(function.length.iter());
            arguments
        }
        Expression::Replace(function) => vec![&function.this, &function.old, &function.new],
        Expression::Left(function) | Expression::Right(function) => {
            vec![&function.this, &function.length]
        }
        Expression::Round(function) => {
            let mut arguments: Vec<&Expression> = vec![&function.this];
            arguments.extend(function.decimals.iter());
            arguments
        }
        Expression::Floor(function) => {
            let mut arguments: Vec<&Expression> = vec![&function.this];
            arguments.extend(function.scale.iter());
            arguments
        }
        Expression::Ceil(function) => {
            let mut arguments: Vec<&Expression> = vec![&function.this];
            arguments.extend(function.decimals.iter());
            arguments
        }
        Expression::Power(function) => vec![&function.this, &function.expression],
        Expression::Trim(function) => {
            let mut arguments: Vec<&Expression> = vec![&function.this];
            arguments.extend(function.characters.iter());
            arguments
        }
        _ => return None,
    };
    Some((node.variant_name().to_owned(), arguments))
}
