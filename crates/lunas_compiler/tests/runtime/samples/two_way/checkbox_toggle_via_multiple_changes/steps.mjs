export default async ({ $, dispatch, expect }) => {
  const input = $("input");
  for (let i = 0; i < 3; i++) {
    input.checked = !input.checked;
    await dispatch(input, "change");
  }
  expect("input").prop("checked", true);
};
