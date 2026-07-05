export default async ({ expect }) => {
  expect("a").attr("href", "/go");
  expect("input").value("hello");
};
