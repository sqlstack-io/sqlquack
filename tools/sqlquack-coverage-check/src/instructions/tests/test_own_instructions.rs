use crate::instructions::_helpers::own_instructions::own_instructions;
use crate::instructions::tests::test_types::ProfileTestCase;

#[test]
fn given_a_callgrind_profile_when_counting_then_only_the_programs_own_self_cost_counts() {
    let test_cases = [
        ProfileTestCase {
            description: "self cost in the program, a call's inclusive line skipped, libc left out",
            profile: "events: Ir\n\
                      ob=(1) /work/target/gate/syntax-coverage\n\
                      fl=(1) a.rs\n\
                      fn=(1) parse\n\
                      10 100\n\
                      +2 20\n\
                      cob=(2) /usr/lib/libc.so.6\n\
                      cfn=(2) malloc\n\
                      calls=3 0\n\
                      12 9000\n\
                      * 3\n\
                      ob=(2)\n\
                      fn=(2)\n\
                      0 9000\n\
                      ob=(1)\n\
                      fn=(3) tokens\n\
                      -1 7\n\
                      totals: 9130\n",
            expected_count: 130,
        },
        ProfileTestCase {
            description: "an object with the same directory but another file name",
            profile: "ob=/work/target/gate/syntax-coverage-old\nfn=a\n1 50\n",
            expected_count: 0,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            own_instructions(test_case.profile, "syntax-coverage"),
            Ok(test_case.expected_count),
            "{}",
            test_case.description
        );
    }
}
