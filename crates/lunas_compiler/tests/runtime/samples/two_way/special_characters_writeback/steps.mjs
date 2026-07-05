export default async ({ setValue, expect }) => {
  await setValue("input", 'a&b<c>"d"');
  expect("span").text('a&b<c>"d"');
};
