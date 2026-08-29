# Events editor: one page per expression, names, pickers, tooltips

- STATUS: OPEN
- PRIORITY: 62
- TAGS: v0.12.0, editor, ui, events

Successor to `20260829-133038`, from the owner's read of the shipped Events
mode. The mode itself is right; what it draws is not finished.

## The expression AST leaves the hierarchy

The AST stays - the owner reads it as the cleaner shape - but it is not a
hierarchy concern. The rail shows ONE row for the expression filter; selecting
it opens the expression editor as a PAGE: the whole tree, indented, each
operator and each leaf editable in place. `InspectorField` is already
node-addressed, so a page of rows can write to several document nodes.

A variable is an IDENTIFIER, not a value. A leaf naming one picks from the
variables in scope the way the VariableSet row does, rather than being typed.

## Handlers get a label

`ScenarioEventConfig.name` is the trigger, so a title is a new optional field.
The tree row reads BOTH: `OnEnter - picket warden wakes`.

## The rest of the surface

- The Add menu splits by view, like Ship does, with names that say what they
  make. `Sequence` is the hard one to figure out and should read like what it
  is.
- A choice with many options is a DROPDOWN, not a wrapping wall of chips - the
  handler trigger has sixteen.
- Every field holding a path gets a pick, and says whether the path resolves.
  Assets belong on the Events screen; drag-and-drop is the shape to aim at.
- TOOLTIPS on most things. The Events screen has to explain what a field means
  without the docs open beside it.
- Every textbox aligned with its unit; every label earning its place. A row
  reading `turret` and nothing else says nothing.

## Done when

- An expression is one row in the tree and one page in the editor, with its
  operators emphasised and its identifiers picked rather than typed.
- A handler can be named, and the tree reads the trigger and the name.
- Add offers what the current view can make, under names that need no glossary.
- A long choice is a dropdown; a path field picks and validates.
- Resting on a field in Events says what it is for.
- Units, boxes and labels line up, and no label is a bare word with no subject.
