// @ts-check
const assert = require("assert");
const createFromBuffer = require("@dprint/formatter").createFromBuffer;
const getBuffer = require("./index").getBuffer;

const formatter = createFromBuffer(getBuffer());

assert.strictEqual(
  formatter.formatText({
    filePath: "file.html",
    fileText: "<div><p>Hello</p><script>let a=1</script><style>a{color:red}</style></div>",
  }),
  "<div>\n  <p>Hello</p>\n  <script>\n    let a = 1;\n  </script>\n  <style>\n    a {\n      color: red;\n    }\n  </style>\n</div>\n",
);

assert.strictEqual(
  formatter.formatText({
    filePath: "file.ts",
    fileText: "const a={b:'c'}",
  }),
  "const a = { b: \"c\" };\n",
);

// a deeply nested input, so a stack that's too small for the recursive printers fails here
const depth = 200;
const nested = "[".repeat(depth) + "]".repeat(depth);
assert.ok(formatter.formatText({ filePath: "file.json", fileText: nested }).length > 0);
