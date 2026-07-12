export default async ({ $, dispatch, expect }) => {
  expect("span").text("0");
  const elA = $(".a");
  elA.checked = true;
  await dispatch(elA, "change");
  const elC = $(".cc");
  elC.checked = true;
  await dispatch(elC, "change");
  expect("span").text("2");
};
