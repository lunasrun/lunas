export default async ({ expect, dispatch }) => {
  expect("input").attr("id", "field1").attr("placeholder", "type here").prop("disabled", true);
  await dispatch("input", "focus");
  expect("input").prop("disabled", false);
};
