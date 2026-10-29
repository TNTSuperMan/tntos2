//@ts-ignore
const font: string = await Bun.file("shnm8x16a.bdf").text();
const bin = new Uint8Array(16 * 128);
const a = start();
a.next();
for (const line of font.split("\n")) {
    a.next(line);
}
//@ts-ignore
await Bun.write("font.bin", bin);
function* get(reg: RegExp): Generator<void, RegExpExecArray, string> {
    while (true) {
        const r = reg.exec(yield);
        if (r) {
            return r;
        }
    }
}
function* start(): Generator<void, void, string> {
    while (true) {
        const char = parseInt((yield* get(/^STARTCHAR ([\da-f]{2})$/))[1]!, 16);
        yield* get(/^BITMAP$/);

        let i = 0;

        while (true) {
            const str = yield;
            if (str === "ENDCHAR") break;
            const data = parseInt(str, 16);
            bin[char * 16 + i] = data;
            i++;
        }
    }
}
