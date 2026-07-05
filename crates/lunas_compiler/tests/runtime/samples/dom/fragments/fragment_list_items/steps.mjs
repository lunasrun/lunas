export default async ({ expect }) => {
  expect("ul").html(
    "<li class=\"one\">one</li><li class=\"two\">two</li><li class=\"three\">three</li>"
  );
};
