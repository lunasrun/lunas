export default async ({ expect }) => {
  // :value binds the IDL property directly (no coercion); the DOM (and the
  // shim) normalize a null/undefined value read-back to "".
  expect("input").value("");
};
