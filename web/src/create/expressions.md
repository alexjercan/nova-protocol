# Variables & expressions

Scenario state lives in ONE flat variable store: string keys, typed values.
[`VariableSet`](../actions/#variableset) writes it,
[`Expression` filters](../filters/#expression) read it,
[`HudReadout`](../actions/#hudreadout) displays it, and the whole store
clears at scenario teardown (nothing persists across scenarios or a retry).
Expressions are a small hand-rolled AST with the usual precedence chain -
this page is its complete grammar.

The chain, from the boolean root down to the atoms:

```text
condition   LessThan | GreaterThan | Equal          (filters only)
expression  Add | Subtract | Term                   (the value root)
term        Multiply | Divide | Factor
factor      Literal | Name | Parens                 (the atoms)
```

Every node is a tuple variant - operands are written positionally, nested
inline. The simplest full chains, worth memorizing:

```ron
Term(Factor(Literal(Number(0.0))))     // the number 0
Term(Factor(Name("beat")))             // read the variable "beat"
```

## Values: the literal types

Three value types (`VariableLiteral`):

| variant | payload | example |
|---|---|---|
| `Number(..)` | 64-bit float | `Literal(Number(4.0))` |
| `String(..)` | string | `Literal(String("act_two"))` |
| `Boolean(..)` | bool | `Literal(Boolean(true))` |

There is no null and no integer type - counters are `Number`s.

## Factors: the atoms

| variant | payload | meaning |
|---|---|---|
| `Literal(<literal>)` | a value | a constant |
| `Name("var")` | a variable key | read that variable; an UNDEFINED name is an evaluation error (the enclosing filter fails closed, a `VariableSet` skips the write) |
| `Parens(<expression>)` | a whole expression | parenthesized subexpression - how you put an `Add` under a `Multiply` |

## Terms: multiply / divide

| variant | operands | semantics |
|---|---|---|
| `Factor(<factor>)` | - | a bare factor |
| `Multiply(<factor>, <term>)` | factor x term | Number x Number = product; Boolean x Boolean = logical AND; anything else = type error |
| `Divide(<factor>, <term>)` | factor / term | Numbers only; dividing by 0.0 is an evaluation error (fails closed, never NaN) |

## Expressions: add / subtract (the value root)

The node `VariableSet.expression` takes.

| variant | operands | semantics |
|---|---|---|
| `Term(<term>)` | - | a bare term |
| `Add(<term>, <expression>)` | term + expression | Number + Number = sum; Boolean + Boolean = logical OR; String + String = concatenation; mixed = type error |
| `Subtract(<term>, <expression>)` | term - expression | Numbers only |

Note the asymmetric arms: the LEFT operand is a term (or factor, one level
down), the RIGHT is a full expression. That is ordinary precedence
plumbing - it is why the shipped increment reads
`Add(Factor(Name("n")), Term(Factor(Literal(Number(1.0)))))`: the left arm
skips straight to a factor, the right arm is a complete expression.

Chains associate RIGHTWARD: `a - b - c` authored as
`Subtract(a, Subtract(b, Term(c)))` computes `a - (b - c)`. Chained `Add` is
safe (associative); for subtraction, use `Parens` to force the grouping you
mean.

A three-term sum, for the record:

```ron
Add(
    Factor(Name("a")),
    Add(
        Factor(Name("b")),
        Term(Factor(Name("c"))),
    ),
)
```

## Conditions: the boolean root

The node an [`Expression` filter](../filters/#expression) wraps. Compares
two value expressions; yields a boolean.

| variant | operands | semantics |
|---|---|---|
| `LessThan(<expr>, <expr>)` | numeric | `l < r`; non-numbers = type error |
| `GreaterThan(<expr>, <expr>)` | numeric | `l > r`; non-numbers = type error |
| `Equal(<expr>, <expr>)` | same type | Numbers compare within epsilon 1e-6 (exact float equality burned an author once); Strings/Booleans compare exactly; mixed types = type error |

There is NO `NotEqual`, `LessOrEqual` or `GreaterOrEqual`. Compose instead:
wrap the filter in [`Conditional(Not(..))`](../filters/#conditional), or
flip the comparison (`>= n` on an integer counter is `> n - 1` - and that
form is the [count-gate](#recipes) convention anyway).

## Queries and watched variables

Queries are typed, read-only world observations. A scenario can expose one as
an auto-updating variable with `watches`:

```ron
watches: [
    (
        variable: "scenario_elapsed",
        query: Scenario((property: Elapsed)),
    ),
    (
        variable: "courier_speed",
        query: Entity((
            filter: (id: "courier"),
            property: Speed,
        )),
    ),
],
```

Use watched values through the normal variable syntax, including HUD readouts:
`Name("scenario_elapsed")`. A watched name is read-only; `VariableSet` on it is
a lint error.

Queries can also be inline expression factors. This takes a one-shot speed
snapshot when the action runs:

```ron
VariableSet((
    key: "speed_at_gate",
    expression: Term(Factor(Query(Entity((
        filter: (id: "courier"),
        property: Speed,
    ))))),
))
```

Supported queries:

| query | result | meaning |
|---|---|---|
| `Scenario((property: Elapsed))` | Number | live, unpaused scenario seconds; resets on teardown |
| `Entity((filter: (id: "..."), property: Speed))` | Number | speed in u/s of exactly one matching entity |

`Entity` is strict-single. Zero matches, multiple matches, or a missing velocity
make the query unavailable. Expressions fail closed. Missing is not zero.

## Recipes

The compositions every shipped scenario is built from. All of them depend on
one rule: **seed every variable in `OnStart`** - expression filters
[fail closed](../filters/#traps-for-the-unwary) on unset names.

Increment a counter (re-evaluated per event, so it accumulates):

```ron
VariableSet((
    key: "crates_recovered",
    expression: Add(Factor(Name("crates_recovered")), Term(Factor(Literal(Number(1.0))))),
))
```

The count-gate + one-shot flag - a second handler (often `OnUpdate`, so it
never depends on handler order) that fires ONCE when the counter crosses a
threshold. Prefer `> n-1` over `== n` on a count gate: a double-fire that
jumps the counter past `n` cannot skip a `>` gate, but sails clean over an
`==` one.

```ron
(
    name: OnUpdate,
    filters: [
        Expression((GreaterThan(
            Term(Factor(Name("crates_recovered"))),
            Term(Factor(Literal(Number(1.0)))),      // fires at 2 or more
        ))),
        Expression((Equal(
            Term(Factor(Name("quota_done"))),
            Term(Factor(Literal(Boolean(false)))),
        ))),
    ],
    actions: [
        VariableSet((key: "quota_done", expression: Term(Factor(Literal(Boolean(true)))))),
        // ... the beat ...
    ],
),
```

A one-shot timed beat (clock threshold + flag):

```ron
filters: [
    Expression((GreaterThan(
        Term(Factor(Name("scenario_elapsed"))),
        Term(Factor(Literal(Number(30.0)))),
    ))),
    Expression((Equal(
        Term(Factor(Name("beat_fired"))),
        Term(Factor(Literal(Number(0.0)))),
    ))),
],
```

A repeating wave - gate on `elapsed > next_at`, re-arm inside the action:

```ron
actions: [
    VariableSet((
        key: "next_at",
        expression: Add(Factor(Name("next_at")), Term(Factor(Literal(Number(30.0))))),
    )),
    // ... spawn the wave ...
],
```

Snapshot the clock to measure "since X": store `scenario_elapsed` into your
own variable when X happens, then gate on
`elapsed > snapshot + grace` via `Add` under `Parens`.

A linear state machine: one numeric `beat` counter, every handler filtered
on `Equal(beat, N)` and ending with `VariableSet(beat, N+1)`. This scales
better than a boolean per step; the Shakedown Run and the Gauntlet are both
built this way (see the scenario-engine chapter of the
[developer book](../../dev/)).

## Traps for the unwary

- Everything fails CLOSED: undefined names, type mismatches and division by
  zero all log an error and make the filter false / skip the write. No
  crash, no default value - the handler just never fires. Seed in
  `OnStart`.
- No boolean node exists at the VALUE level beyond the `Add`/`Multiply`
  overloads (OR / AND on booleans). For filter logic, prefer multiple
  `filters` entries (already ANDed) and
  [`Conditional`](../filters/#conditional) for OR / NOT.
- RON's recursion limit (128) bounds nesting depth; a pathological
  expression is a parse error, not a hang.
