export default async ({ $$, equal }) => {
  equal($$(".a").length, 2);
  equal($$(".b").length, 2);
};
