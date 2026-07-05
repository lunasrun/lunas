export default async ({ expect }) => {
  expect("input").prop("disabled", false);
  expect("input").attr("required", "false");
};
