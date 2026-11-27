# @dprint/tidy

npm distribution of [dprint-plugin-tidy](https://github.com/dprint/dprint-plugin-tidy) which is an adapter plugin for [tidy](https://github.com/lukeed/tidy).

Use this with [@dprint/formatter](https://github.com/dprint/js-formatter) or just use @dprint/formatter and download the [dprint-plugin-tidy Wasm file](https://github.com/dprint/dprint-plugin-tidy/releases).

## Example

```ts
import { createFromBuffer } from "@dprint/formatter";
import { getBuffer } from "@dprint/tidy";

const formatter = createFromBuffer(getBuffer());

console.log(
  formatter.formatText({
    filePath: "file.html",
    fileText: "<div><p>Hello</p></div>",
  }),
);
```
