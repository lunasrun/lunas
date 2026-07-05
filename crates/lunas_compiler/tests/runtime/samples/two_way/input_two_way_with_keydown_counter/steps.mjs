export default async ({ dispatch, setValue, expect }) => {
  await dispatch("input", "keydown", { key: "a" });
  expect("span").text("s=1 t=");
  await setValue("input", "a");
  expect("span").text("s=1 t=a");
};
