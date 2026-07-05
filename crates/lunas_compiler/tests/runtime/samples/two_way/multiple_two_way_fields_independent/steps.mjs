export default async ({ setValue, expect }) => {
  expect("p").text("a b");
  await setValue(".first", "alice");
  expect("p").text("alice b");
  await setValue(".last", "smith");
  expect("p").text("alice smith");
};
