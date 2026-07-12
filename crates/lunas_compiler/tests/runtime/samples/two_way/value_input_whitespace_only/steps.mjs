export default async ({ setValue, expect }) => {
  await setValue("input", "   ");
  expect("span").text("[   ]");
};
