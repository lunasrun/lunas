export default async ({ $, click, expect }) => {
  expect($("a")).attr("href", "/a");
  await click("button");
  expect($("a")).attr("href", "/b");
};
