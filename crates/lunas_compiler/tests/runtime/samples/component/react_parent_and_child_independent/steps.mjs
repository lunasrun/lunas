export default async ({ $, click, expect }) => {
  const span = $("span");
  expect(span).text("1+0");
  await click(".pbtn");
  expect(span).text("2+0");
  await click(".cbtn");
  expect(span).text("2+1");
  await click(".pbtn");
  await click(".cbtn");
  expect(span).text("3+2");
};
