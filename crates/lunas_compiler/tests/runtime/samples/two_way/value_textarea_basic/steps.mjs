export default async ({ setValue, expect }) => {
  expect("span").text("start");
  await setValue("textarea", "end");
  expect("span").text("end");
};
