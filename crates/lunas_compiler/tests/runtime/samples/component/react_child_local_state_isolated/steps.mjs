export default async ({ $, click, expect }) => {
  expect($(".p")).text("0");
  expect($(".c")).text("0");
  await click("button");
  expect($(".c")).text("1");
  // parent state untouched by child-local mutation
  expect($(".p")).text("0");
};
