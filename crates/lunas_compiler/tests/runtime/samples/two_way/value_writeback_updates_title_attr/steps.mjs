export default async ({ setValue, expect }) => {
  expect("input").attr("title", "start");
  await setValue("input", "end");
  expect("input").attr("title", "end");
};
