export default async ({ $, dispatch, expect }) => {
  expect("p").count(0);
  const input = $("input");
  input.checked = true;
  await dispatch(input, "change");
  expect("p").text("visible content");
  input.checked = false;
  await dispatch(input, "change");
  expect("p").count(0);
};
