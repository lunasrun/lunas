export default async ({ $, click, tick, expect }) => {
  expect(".box").html("<button>t</button>");
  await click("button");
  expect(".box").html("<button>t</button><span>HERE</span>");
  await click("button");
  expect(".box").html("<button>t</button>");
};
