export default async ({ setValue, click, expect }) => {
  expect("p").text("keep typing");
  await setValue("textarea", "more notes");
  expect("p").text("more notes");
  await click("button");
  expect("p").text("");
};
