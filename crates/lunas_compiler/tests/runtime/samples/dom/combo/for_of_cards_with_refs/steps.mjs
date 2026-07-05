export default async ({ $$, click, equal }) => {
  // Each Card in the :for list owns its OWN `self` ref -- component-scoped, so
  // unlike a ref declared directly inside a :for item's OWN template (which
  // shares one variable across iterations), each mounted Card instance here
  // has an independent `self`.
  const labels = $$(".card-label");
  equal(labels.length, 3);
  await click(labels[1]);
  equal(labels[0].getAttribute("data-clicked"), null);
  equal(labels[1].getAttribute("data-clicked"), "yes");
  equal(labels[2].getAttribute("data-clicked"), null);
};
