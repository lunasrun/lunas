export default async ({ $, setValue, expect }) => {
  expect($("span")).text("hello");
  await setValue("input", "world");
  expect($("span")).text("world");
};
