export default async ({ expect }) => {
  // Outer is itself a fragment (2 top-level nodes: <h1> + <Inner/>), and
  // Inner is also a fragment (2 top-level <p>s) -- fragments nest cleanly.
  expect("h1").text("outer heading");
  expect(".inner-a").text("inner a");
  expect(".inner-b").text("inner b");
};
