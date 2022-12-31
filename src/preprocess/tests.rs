use super::*;
use assert_matches::assert_matches;

mod dollarsplit {
    use super::*;
    #[derive(Debug, Clone)]
    struct Soll {
        lico: LiCo,
        content: &'static str,
    }

    macro_rules! test_case {
    ($name:ident : $input:tt $( =>)? $( ($lineno:literal, $column:literal, $content:literal) ),* $(,)? ) => {
        #[test]
        fn $name () {
            const LIT: &str = $input;
            let soll: &[Soll] = &[
                $( Soll {
                    lico: LiCo {
                        lineno: $lineno,
                        column: $column,
                    },
                    content: $content,
                }),*
            ];
            let ist = Vec::from_iter(dollar_split_tags_iter(LIT));
            ist.iter().zip(soll.iter()).enumerate().for_each(|(idx, (ist, soll))| {
                // assert!(lico > previous_lico);
                dbg!((&idx, &ist, &soll));
                if idx & 0x1 == 0 {
                    assert_matches!(ist.which, Dollar::Start(s) => {
                        assert_eq!(ist.lico, soll.lico);
                        assert_eq!(s, soll.content);
                        // assert_eq!(LIT.match_indices(s).filter(|(offset, x)| offset == soll.byte_offset).count(), 1);
                    })

                } else {
                    assert_matches!(ist.which, Dollar::End(s) => {
                        assert_eq!(ist.lico, soll.lico);
                        assert_eq!(s, soll.content);
                        // assert_eq!(LIT.match_indices(s).filter(|(offset, x)| offset == soll.byte_offset).count(), 1);
                    })
                }

            })

        }
    };
}

    test_case!(bare:
    r###"a b c"###
    );

    test_case!(oneline:
    r###"a $b$ c"### => (0,2, "$"), (0,4, "$")
    );

    test_case!(oneline_unclosed:
        r###"a $b c"### => (0,2,"$"), (0,7,"")
    );

    test_case!(dollar_block_1:
    r###"
$$
\epsilon
$$
"### => (1,1, "$$"), (3,1, "$$"));

    test_case!(pre_block_w_unclosed_inlines:
r###"
$a
<pre>
\epsilon
</pre>
$4
"### => (1,0, "$"), (1,3, ""), (5,0,"$"), (5,2, ""));

    test_case!(all_in_code_block:
r###"
```bash
$ foo $ $$ $?
```
"###
    );

    test_case!(
        iter_over_empty_intra_line_sequences: "foo $$_$$ bar" => (0,4,"$"),(0,5,"$"),(0,7,"$"),(0,8,"$")
    );
}

mod sequester {

    use super::*;

    struct SollSequester {
        keep: bool,
        bytes: std::ops::Range<usize>,
        content: &'static str,
    }

    const K: bool = true;
    const R: bool = false;
    macro_rules! test_sequester {
    ($name:ident : $input:tt $( =>)? $( ($keep:ident, $byte_start:literal .. $byte_end:literal, $content:literal) ),* ) => {
        #[test]
        fn $name () {
            const LIT: &str = $input;
            let soll: &[SollSequester] = &[
                $( SollSequester {
                    keep: $keep,
                    bytes: $byte_start .. $byte_end,
                    content: $content,
                }),*
            ];
            let split_points_iter = dollar_split_tags_iter(LIT);
            let ist = iter_over_dollar_encompassed_blocks(LIT, split_points_iter);
            let ist = Vec::<Tagged<'_>>::from_iter(ist);
            ist.iter().zip(soll.iter()).enumerate().for_each(|(_idx, (ist, soll)): (usize, (_, &SollSequester))| {
                assert_eq!(&LIT[soll.bytes.clone()], soll.content, "Test case integrity violated");
                match dbg!(&ist) {
                    Tagged::Replace(_c) => { assert!(!soll.keep); }
                    Tagged::Keep(_c) => { assert!(soll.keep); }
                }
                let content: &Content<'_> = ist.as_ref();
                assert_eq!(&content.s[..], &soll.content[..]);
                assert_eq!(&content.s[..], &LIT[soll.bytes.clone()]);
            })

        }
    };

}

    test_sequester!(
    singlje_x:
        "x: $1$" =>
    (K, 0..3, "x: "),
    (R, 3..6, "$1$"));

    test_sequester!(
    doublje_x_y:
        "x:$1$y:$2$" =>
    (K, 0..2, "x:"),
    (R, 2..5, "$1$"),
    (K, 5..7, "y:"),
    (R, 7..10, "$2$"));

    test_sequester!(onelinje:
    "$1$ $2$ $3$" =>
(R, 0..3, "$1$"),
(K, 3..4, " "),
(R, 4..7, "$2$"),
(K, 7..8, " "),
(R, 8..11, "$3$"));

    test_sequester!(oneblockje:
r#"$$
1
$$"# =>
(R, 0..7, r#"$$
1
$$"#));

    test_sequester!(oneblockje_w_prefix:
    r#"Hello, there is a block
$$
1
$$"# =>
    (K, 0..24, r#"Hello, there is a block
"#),
    (R, 24..31, "$$
1
$$")
    );

    test_sequester!(nope:
    r####"# abc

Hello, the block is a myth!
        
"#### =>
    (K, 0..44, r####"# abc

Hello, the block is a myth!
        
"####)
    );
}
