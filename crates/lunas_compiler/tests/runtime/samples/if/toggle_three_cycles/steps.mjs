export default async ({ click, expect }) => {
  for (let i = 0; i < 3; i++) {
    expect("span").count(0);
    await click("button");
    expect("span").count(1);
    await click("button");
  }
  expect("span").count(0);
};
