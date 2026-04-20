import { assert, assertPoint, assertThrowsWithMessage, demo } from "../support/index.mjs";
import { wireErr, wireOk } from "@boltffi/runtime";

export async function run() {
  assert.equal(demo.safeDivide(10, 2), 5);
  assertThrowsWithMessage(() => demo.safeDivide(1, 0), "division by zero");
  assert.equal(demo.safeSqrt(9), 3);
  assertThrowsWithMessage(() => demo.safeSqrt(-1), "negative input");
  assertPoint(demo.parsePoint("1.5, 2.5"), { x: 1.5, y: 2.5 });
  assertThrowsWithMessage(() => demo.parsePoint("wat"), "expected format");
  assert.equal(demo.alwaysOk(21), 42);
  assertThrowsWithMessage(() => demo.alwaysErr("boom"), "boom");
  assert.equal(demo.resultToString(wireOk(7)), "ok: 7");
  assert.equal(demo.resultToString(wireErr("bad")), "err: bad");
  assert.equal(demo.divide(10, 2), 5);
  assertThrowsWithMessage(() => demo.divide(10, 0), "division by zero");
  assert.equal(demo.parseInt("42"), 42);
  assertThrowsWithMessage(() => demo.parseInt("nope"), "invalid integer");
  assert.equal(demo.validateName("Ali"), "Hello, Ali!");
  assertThrowsWithMessage(() => demo.validateName(""), "name cannot be empty");
}
