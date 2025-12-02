import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { publishTopic } from './support.mjs'

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

async function mainText(driver) {
  let last
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try {
      return (await driver.findElement(By.css('main')).getText()).replace(/\s+/g, ' ')
    } catch (err) {
      last = err
      await new Promise((resolve) => setTimeout(resolve, 250))
    }
  }
  throw last
}

async function clickWhenReady(driver, locator, message) {
  await driver.wait(until.elementLocated(locator), 10000, message)
  let last
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try {
      await driver.findElement(locator).click()
      return
    } catch (err) {
      last = err
      await new Promise((resolve) => setTimeout(resolve, 250))
    }
  }
  throw last
}

async function register(driver, username) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 10000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 10000)
}

const fold = By.xpath("//button[normalize-space(text())='Hide replies']")

async function run() {
  const driver = await buildDriver()
  const suffix = Date.now().toString(36)
  const name = `e2e_thread_${suffix}`
  const title = `A subject with a thread ${suffix}`
  const parent = `A parent remark ${suffix}`
  const child = `A nested reply ${suffix}`
  try {
    await register(driver, name)

    await driver.get(`${baseUrl}/s/general`)
    await driver.wait(
      until.elementLocated(By.xpath("//button[text()='New topic']")),
      10000,
      'the new topic control should appear once posting is known to be allowed',
    )
    await driver.findElement(By.xpath("//button[text()='New topic']")).click()
    await driver.wait(until.elementLocated(By.name('title')), 10000)
    await driver.findElement(By.name('title')).sendKeys(title)
    await driver.findElement(By.name('body')).sendKeys('A body for the thread spec.')
    await clickWhenReady(driver, By.xpath("//button[text()='Post']"), 'the post control')
    await publishTopic(title)
    await driver.get(`${baseUrl}/s/general`)
    await driver.wait(until.elementLocated(By.linkText(title)), 10000)
    await driver.findElement(By.linkText(title)).click()
    await driver.wait(until.elementLocated(By.name('comment-body')), 10000)

    const withTime = await mainText(driver)
    assert(
      withTime.includes('just now') || / ago\b/.test(withTime),
      `a subject should say when it was written, saw: ${withTime}`,
    )

    await driver.findElement(By.name('comment-body')).sendKeys(parent)
    await clickWhenReady(driver, By.xpath("//button[text()='Post comment']"), 'the comment control')
    await driver.wait(
      until.elementLocated(By.xpath(`//p[contains(., '${parent}')]`)),
      10000,
    )

    const noFoldYet = await driver.findElements(fold)
    assert(
      noFoldYet.length === 0,
      'a remark with no replies should offer nothing to fold away',
    )

    await clickWhenReady(driver, By.xpath("//button[normalize-space(text())='Reply']"), 'the reply control')
    const replyBox = await driver.wait(until.elementLocated(By.name('reply-body')), 10000)
    await replyBox.sendKeys(child)
    await clickWhenReady(driver, By.xpath("//button[text()='Post reply']"), 'the post reply control')
    await driver.wait(
      until.elementLocated(By.xpath(`//p[contains(., '${child}')]`)),
      10000,
    )

    await driver.wait(
      until.elementLocated(fold),
      10000,
      'a remark with a reply should offer to fold it away',
    )
    const open = await driver.findElement(fold)
    assert(
      (await open.getAttribute('aria-expanded')) === 'true',
      'a thread starts open, and says so',
    )

    await open.click()
    await driver.wait(
      async () => !(await mainText(driver)).includes(child),
      10000,
      'folding should put the reply away',
    )
    const folded = await mainText(driver)
    assert(folded.includes(parent), 'folding a reply must not take its parent with it')
    assert(
      folded.includes('Show 1 reply'),
      `the control should say how many are hidden, saw: ${folded}`,
    )

    const shut = await driver.findElement(
      By.xpath("//button[normalize-space(text())='Show 1 reply']"),
    )
    assert(
      (await shut.getAttribute('aria-expanded')) === 'false',
      'a folded thread says it is folded',
    )

    await shut.click()
    await driver.wait(
      async () => (await mainText(driver)).includes(child),
      10000,
      'unfolding should bring the reply back',
    )

    console.log('e2e: thread reading flow passed')
  } finally {
    await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
