export default async ({ click, expect }) => {
  expect(".even").count(1);
  for (let i = 0; i < 5; i++) {
    await click("button");
  }
  // After 5 toggles starting from even: odd,even,odd,even,odd -> odd
  expect(".odd").count(1);
  expect(".even").count(0);
};
