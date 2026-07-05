export default async ({ click, setValue, expect }) => {
  expect("input").count(1);
  expect("input").value("edit me");
  await setValue("input", "changed");
  expect("input").value("changed");
  await click("button");
  expect("p").text("Locked: changed");
  expect("input").count(0);
};
