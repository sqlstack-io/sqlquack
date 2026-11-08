// Evaluates DuckDB's railroad diagram scripts into JSON grammar trees.
//
// Usage: node railroad.js <railroad.js of the docs version> <page script>...
// Prints {"<page script>": {"<element id>": <tree>, ...}, ...} on stdout. The docs' node
// constructors are replaced by ones that build plain trees; an Expandable part becomes a named
// definition, expanded once per diagram, with a reference wherever it recurs.
"use strict";

const fs = require("fs");
const vm = require("vm");

function node(kind, fields) {
  const value = Object.assign({ kind }, fields);
  Object.defineProperty(value, "toString", { value: () => JSON.stringify(value) });
  return value;
}

function items(value) {
  if (!Array.isArray(value)) {
    return [part(value)];
  }
  return value.flat(Infinity).filter((item) => item !== undefined && item !== null).map(part);
}

function part(value) {
  if (typeof value === "string") {
    return node("keyword", { text: value });
  }
  if (value.kind !== undefined) {
    return value;
  }
  if (value instanceof classes.Skip || value instanceof classes.Comment) {
    return node("sequence", { items: [] });
  }
  if (value instanceof classes.NonTerminal) {
    return node("placeholder", { text: value.text });
  }
  if (value instanceof classes.Terminal) {
    return node("keyword", { text: value.text });
  }
  throw new Error(`unknown railroad object ${value.constructor.name}`);
}

// The docs' own node classes, which a script may instantiate directly with `new`.
let classes = {};

function constructors(state) {
  return {
    Diagram: (parts) => node("sequence", { items: items(parts) }),
    Sequence: (parts) => node("sequence", { items: items(parts) }),
    Stack: (parts) => node("sequence", { items: items(parts) }),
    AutomaticStack: (parts) => node("sequence", { items: items(parts) }),
    Optional: (part) => node("optional", { items: items(part) }),
    OptionalSequence: (parts) => node("optional_sequence", { items: items(parts) }),
    OneOrMore: (part, repeat) => node("one_or_more", { items: items(part), separator: separator(repeat) }),
    ZeroOrMore: (part, repeat) => node("zero_or_more", { items: items(part), separator: separator(repeat) }),
    Choice: (normal, parts) => node("choice", { items: items(parts) }),
    Expression: (text = "expr") => node("placeholder", { text }),
    Keyword: (text) => node("keyword", { text }),
    Expandable: (text, options, name, expand) => {
      if (!state.definitions.has(name) && !state.expanding.has(name)) {
        state.expanding.add(name);
        state.definitions.set(name, node("sequence", { items: items(expand(options)) }));
        state.expanding.delete(name);
      }
      return node("definition", { text, name });
    },
  };
}

function separator(repeat) {
  return repeat === undefined || repeat === null ? null : part(repeat);
}

function evaluate(railroad, page) {
  const state = { definitions: new Map(), expanding: new Set(), diagrams: {} };
  const elements = new Map();
  const element = (id) => {
    if (!elements.has(id)) {
      elements.set(id, {
        classList: { add() {}, remove() {} },
        set innerHTML(text) {
          state.diagrams[id] = JSON.parse(text);
        },
      });
    }
    return elements.get(id);
  };
  const context = vm.createContext({
    window: { innerWidth: 800, addEventListener() {} },
    document: { getElementById: element, addEventListener() {}, querySelector: () => null },
    console,
  });
  vm.runInContext(railroad, context);
  classes = {
    Skip: context.Skip,
    Comment: context.Comment,
    Terminal: context.Terminal,
    NonTerminal: context.NonTerminal,
  };
  Object.assign(context, constructors(state));
  vm.runInContext(fs.readFileSync(page, "utf8"), context);
  const definitions = {};
  for (const [name, tree] of state.definitions) {
    definitions[name] = tree;
  }
  return { diagrams: state.diagrams, definitions };
}

const [railroadPath, ...pages] = process.argv.slice(2);
const railroad = fs.readFileSync(railroadPath, "utf8");
const result = {};
for (const page of pages) {
  result[page] = evaluate(railroad, page);
}
process.stdout.write(JSON.stringify(result, null, 1));
