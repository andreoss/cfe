import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'

const baseUrl = process.env.BASE_URL ?? 'http://127.0.0.1:58081'
const chromedriverPath = process.env.CHROMEDRIVER_PATH

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

async function buildDriver() {
  const options = new chrome.Options()
  options.addArguments('--headless=new', '--no-sandbox', '--disable-gpu')
  const builder = new Builder().forBrowser('chrome').setChromeOptions(options)
  if (chromedriverPath) {
    const service = new chrome.ServiceBuilder(chromedriverPath)
    builder.setChromeService(service)
  }
  return builder.build()
}

async function run() {
  const driver = await buildDriver()
  const suffix = Date.now().toString(36)
  const username = `e2e_feed_${suffix}`
  const topicTitle = `Syndicated subject ${suffix}`
  const tag = `feedtag${suffix}`
  try {
    await driver.get(`${baseUrl}/register`)
    await driver.wait(until.elementLocated(By.name('username')), 5000)
    await driver.findElement(By.name('username')).sendKeys(username)
    await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
    await driver.findElement(By.name('password')).sendKeys('correcthorse')
    await driver.findElement(By.css('button[type="submit"]')).click()
    await driver.wait(until.urlIs(`${baseUrl}/`), 5000)

    await driver.get(`${baseUrl}/s/general`)
    await driver.wait(until.elementLocated(By.xpath("//button[text()='New topic']")), 5000)
    await driver.findElement(By.xpath("//button[text()='New topic']")).click()
    await driver.findElement(By.name('title')).sendKeys(topicTitle)
    await driver.findElement(By.name('body')).sendKeys('Body for the feeds e2e spec.')
    await driver.findElement(By.name('tags')).sendKeys(tag)
    await driver.findElement(By.xpath("//button[text()='Post']")).click()
    await driver.wait(until.elementLocated(By.linkText(topicTitle)), 5000)

    const sectionFeed = await driver.findElement(By.linkText('Atom feed'))
    const sectionFeedHref = await sectionFeed.getAttribute('href')
    assert(
      sectionFeedHref.includes('/api/sections/general/feed'),
      `section feed link should point at the section feed, was ${sectionFeedHref}`,
    )

    const sectionXml = await driver.executeAsyncScript(
      `const cb = arguments[arguments.length - 1];
       fetch(arguments[0]).then(r => r.text().then(t => cb({ status: r.status, type: r.headers.get('content-type'), body: t })));`,
      sectionFeedHref,
    )
    assert(sectionXml.status === 200, 'the section feed should load')
    assert(
      sectionXml.type.includes('atom+xml'),
      `the feed should be served as atom, was ${sectionXml.type}`,
    )
    assert(sectionXml.body.includes('<feed'), 'the feed should be an atom document')
    assert(
      sectionXml.body.includes(topicTitle),
      'the new topic should appear in the section feed',
    )
    assert(
      sectionXml.body.includes(`<name>${username}</name>`),
      'the feed entry should name its author',
    )

    await driver.get(`${baseUrl}/tag/${tag}`)
    await driver.wait(until.elementLocated(By.linkText('Atom feed')), 5000)
    const tagFeedHref = await driver.findElement(By.linkText('Atom feed')).getAttribute('href')
    assert(
      tagFeedHref.includes(`/api/tags/${tag}/feed`),
      `tag feed link should point at the tag feed, was ${tagFeedHref}`,
    )

    const tagXml = await driver.executeAsyncScript(
      `const cb = arguments[arguments.length - 1];
       fetch(arguments[0]).then(r => r.text().then(t => cb({ status: r.status, body: t })));`,
      tagFeedHref,
    )
    assert(tagXml.status === 200, 'the tag feed should load')
    assert(tagXml.body.includes(topicTitle), 'the tagged topic should appear in the tag feed')

    console.log('e2e: feeds flow passed')
  } finally {
    await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
