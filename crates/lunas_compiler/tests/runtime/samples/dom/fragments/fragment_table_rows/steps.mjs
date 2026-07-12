export default async ({ expect }) => {
  // A fragment's rows are direct children of the parent <table> -- no <div>
  // wrapper is inserted, which would otherwise be invalid HTML there.
  expect("table").html("<tr class=\"r1\"><td>1</td></tr><tr class=\"r2\"><td>2</td></tr>");
};
