export default async ({ $, dispatch, expect }) => {
  const a = $(".a");
  a.checked = true;
  await dispatch(a, "change");
  expect("span").text("a=true b=false");
};
