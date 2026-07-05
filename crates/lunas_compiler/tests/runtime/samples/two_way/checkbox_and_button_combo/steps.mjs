export default async ({ $, click, dispatch, expect }) => {
  const input = $("input");
  input.checked = true;
  await dispatch(input, "change");
  expect("span").text("true");
  await click("button");
  expect("span").text("false");
  expect("input").prop("checked", false);
};
