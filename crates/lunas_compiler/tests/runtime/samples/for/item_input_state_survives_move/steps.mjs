export default async ({ $$, click, setValue, equal }) => {
  const inputs = $$("input.f");
  await setValue(inputs[0], "one");
  await setValue(inputs[2], "three");
  await click(".rev");
  const now = $$("input.f");
  // id1 now last, id3 now first; values travel with the node
  equal(now[2].value, "one");
  equal(now[2].getAttribute("data-id"), "1");
  equal(now[0].value, "three");
  equal(now[0].getAttribute("data-id"), "3");
};
