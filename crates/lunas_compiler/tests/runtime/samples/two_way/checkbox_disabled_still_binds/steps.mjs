export default async ({ $, dispatch, expect }) => {
  expect("input").prop("disabled", false);
  const input = $("input");
  input.checked = true;
  await dispatch(input, "change");
  expect("span").text("true");
};
