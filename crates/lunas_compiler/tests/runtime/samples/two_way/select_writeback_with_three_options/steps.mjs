export default async ({ setValue, expect }) => {
  expect("p").text("chosen: banana");
  await setValue("select", "cherry");
  expect("p").text("chosen: cherry");
  await setValue("select", "apple");
  expect("p").text("chosen: apple");
};
