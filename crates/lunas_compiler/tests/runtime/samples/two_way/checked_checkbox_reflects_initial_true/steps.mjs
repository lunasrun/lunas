export default async ({ $, dispatch, expect }) => {
  expect("input").prop("checked", true);
  expect("span").text("true");
  const input = $("input");
  input.checked = false;
  await dispatch(input, "change");
  expect("span").text("false");
};
