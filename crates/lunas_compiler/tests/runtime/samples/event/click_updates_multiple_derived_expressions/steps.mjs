export default async ({ click, expect }) => {
  expect("button").text("n=0 sq=0 even=true");
  await click("button");
  expect("button").text("n=1 sq=1 even=false");
  await click("button");
  expect("button").text("n=2 sq=4 even=true");
};
