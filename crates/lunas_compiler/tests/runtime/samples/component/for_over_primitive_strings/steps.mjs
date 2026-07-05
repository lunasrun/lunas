// `:for` over PRIMITIVE string items each mounting a child component. Regression
// guard: this must render without the "_children on string" crash.
export default async ({ $$, expect }) => {
  expect("span").count(3);
  expect($$("span")[0]).text("a");
  expect($$("span")[1]).text("b");
  expect($$("span")[2]).text("c");
};
