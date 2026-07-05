export default async ({ setValue, expect }) => {
  await setValue(".a", "changed");
  expect(".b").value("changed");
  await setValue(".b", "again");
  expect(".a").value("again");
};
