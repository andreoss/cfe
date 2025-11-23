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

async function register(driver, username) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 5000)
}

function kindButton(kind) {
  return By.xpath(`(//button[starts-with(normalize-space(.), '${kind} ')])[1]`)
}

async function kindText(driver, kind) {
  const el = await driver.findElement(kindButton(kind))
  return (await el.getText()).trim()
}

async function waitForKind(driver, kind, expected) {
  await driver.wait(async () => (await kindText(driver, kind)) === expected, 5000)
}

async function run() {
  const first = await buildDriver()
  const suffix = Date.now().toString(36)
  const firstName = `e2e_rx_a_${suffix}`
  const secondName = `e2e_rx_b_${suffix}`
  const topicTitle = `Reactable subject ${suffix}`
  let second
  let anon
  try {
    await register(first, firstName)

    await first.get(`${baseUrl}/s/general`)
    await first.wait(until.elementLocated(By.xpath("//button[text()='New topic']")), 5000)
    await first.findElement(By.xpath("//button[text()='New topic']")).click()
    await first.wait(until.elementLocated(By.name('title')), 10000)
    await first.findElement(By.name('title')).sendKeys(topicTitle)
    await first.findElement(By.name('body')).sendKeys('Body for the reactions e2e spec.')
    await first.findElement(By.xpath("//button[text()='Post']")).click()
    await publishTopic(topicTitle)
    await first.get(`${baseUrl}/s/general`)
    await first.wait(until.elementLocated(By.linkText(topicTitle)), 5000)
    await first.findElement(By.linkText(topicTitle)).click()
    await first.wait(until.elementLocated(By.name('comment-body')), 5000)
    const topicUrl = await first.getCurrentUrl()

    await first.wait(until.elementLocated(kindButton('like')), 5000)
    await waitForKind(first, 'like', 'like 0')

    await first.findElement(kindButton('like')).click()
    await waitForKind(first, 'like', 'like 1')

    const mine = await first.findElements(By.css('button.mine'))
    assert(mine.length > 0, 'the chosen reaction should be marked as mine')

    await first.navigate().refresh()
    await first.wait(until.elementLocated(kindButton('like')), 5000)
    await waitForKind(first, 'like', 'like 1')

    await first.findElement(kindButton('agree')).click()
    await waitForKind(first, 'agree', 'agree 1')
    await waitForKind(first, 'like', 'like 0')

    second = await buildDriver()
    await register(second, secondName)
    await second.get(topicUrl)
    await second.wait(until.elementLocated(kindButton('agree')), 5000)
    await waitForKind(second, 'agree', 'agree 1')
    await second.findElement(kindButton('agree')).click()
    await waitForKind(second, 'agree', 'agree 2')

    await first.navigate().refresh()
    await first.wait(until.elementLocated(kindButton('agree')), 5000)
    await waitForKind(first, 'agree', 'agree 2')

    await first.findElement(kindButton('agree')).click()
    await waitForKind(first, 'agree', 'agree 1')
    const stillMine = await first.findElements(By.css('button.mine'))
    assert(stillMine.length === 0, 'clearing your reaction should unmark it')

    anon = await buildDriver()
    await anon.get(topicUrl)
    await anon.wait(until.elementLocated(kindButton('agree')), 5000)
    await waitForKind(anon, 'agree', 'agree 1')
    const anonButton = await anon.findElement(kindButton('agree'))
    assert(
      (await anonButton.getAttribute('disabled')) !== null,
      'a signed-out visitor must not be able to react',
    )

    console.log('e2e: reactions flow passed')
  } finally {
    await first.quit()
    if (second) await second.quit()
    if (anon) await anon.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
