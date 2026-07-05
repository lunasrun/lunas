export default async ({ expect }) => {
  expect("input").prop("disabled", true);
  expect("input").prop("readOnly", false);
  expect("input").attr("required", "true");
};
