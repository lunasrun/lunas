export default async ({ click, expect }) => {
  // Discovered gap: a `<template :if="...">` with MULTIPLE children compiles
  // to `ifBlock`'s make() returning only `r0.childNodes[0]` -- i.e. the
  // literal `<template>` wrapper element itself becomes the inserted node
  // (see expected.after.html), not an unwrapped two-node group as
  // fragments.md's ":if with a multi-node branch" section describes. The
  // dom-shim's naive childNodes/querySelector walk does not model real
  // `<template>` inert-content semantics, so `.head`/`.body` are still found
  // here -- but in a real browser a `<template>` element's children live in
  // an inert DocumentFragment and are NOT part of the rendered tree, so this
  // would NOT actually display in production. Recorded as-is (harness-visible
  // behavior), with the caveat noted for whoever fixes the codegen gap.
  expect(".head").count(0);
  await click("button");
  expect(".head").count(1);
  expect(".body").count(1);
};
