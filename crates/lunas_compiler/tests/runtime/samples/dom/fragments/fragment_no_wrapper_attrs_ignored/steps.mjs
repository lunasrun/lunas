export default async ({ expect }) => {
  // A fragment has no single root to attach attributes to, so attrs put on
  // the <Multi/> call site are ignored (fragments.md): neither top-level <p>
  // ends up with class="ignored-attr" or title="also-ignored".
  expect(".one").text("one");
  expect(".two").text("two");
  expect(".one").attr("title", null);
};
