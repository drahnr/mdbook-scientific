use super::*;

const TESTCASE: &str = r###"

# Hello there

I link $f$ and I $x$ but `not`
so `$nested`. $x = y$.

```sh
$ foo
bar
baz
```

$$ref:fxblck
a = sqrt(2)
$$

As seen in $ref:fxblck$ yada.

"###;

const OUTPUT_MARKDOWN: &str = r###"




"###;

#[test]
fn end2end() {}
