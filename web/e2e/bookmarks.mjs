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

async function register(driver, username) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 5000)
}

async function bookmarksText(driver) {
  await driver.get(`${baseUrl}/bookmarks`)
  await driver.wait(until.elementLocated(By.css('main')), 5000)
  await driver.sleep(500)
  return driver.findElement(By.css('main')).getText()
}

async function run() {
  const owner = await buildDriver()
  const suffix = Date.now().toString(36)
  const ownerName = `e2e_bm_a_${suffix}`
  const otherName = `e2e_bm_b_${suffix}`
  const topicTitle = `Keepable subject ${suffix}`
  let other
  try {
    await register(owner, ownerName)

    let text = await bookmarksText(owner)
    assert(text.includes('No saved topics.'), 'a new user should have no saved topics')

    await owner.get(`${baseUrl}/s/general`)
    await owner.wait(until.elementLocated(By.xpath("//button[text()='New topic']")), 5000)
    await owner.findElement(By.xpath("//button[text()='New topic']")).click()
    await owner.findElement(By.name('title')).sendKeys(topicTitle)
    await owner.findElement(By.name('body')).sendKeys('Body for the bookmarks e2e spec.')
    await owner.findElement(By.xpath("//button[text()='Post']")).click()
    await owner.wait(until.elementLocated(By.linkText(topicTitle)), 5000)
    await owner.findElement(By.linkText(topicTitle)).click()
    await owner.wait(until.elementLocated(By.name('comment-body')), 5000)
    const topicUrl = await owner.getCurrentUrl()
    await owner.wait(
      until.elementLocated(By.xpath("//button[normalize-space(text())='Save topic']")),
      5000,
    )
    await owner.findElement(By.xpath("//button[normalize-space(text())='Save topic']")).click()
    await owner.wait(
      until.elementLocated(By.xpath("//button[normalize-space(text())='Unsave topic']")),
      5000,
    )

    text = await bookmarksText(owner)
    assert(text.includes(topicTitle), 'the saved topic should be listed')
    assert(!text.includes('No saved topics.'), 'the empty message should be gone')
    await owner.get(topicUrl)
    await owner.wait(
      until.elementLocated(By.xpath("//button[normalize-space(text())='Unsave topic']")),
      5000,
    )
    other = await buildDriver()
    await register(other, otherName)
    const otherText = await bookmarksText(other)
    assert(
      otherText.includes('No saved topics.'),
      'another user must not see someone elses saved topics',
    )
    await other.get(topicUrl)
    await other.wait(
      until.elementLocated(By.xpath("//button[normalize-space(text())='Save topic']")),
      5000,
    )

    await owner.get(topicUrl)
    await owner.wait(
      until.elementLocated(By.xpath("//button[normalize-space(text())='Unsave topic']")),
      5000,
    )
    await owner.findElement(By.xpath("//button[normalize-space(text())='Unsave topic']")).click()
    await owner.wait(
      until.elementLocated(By.xpath("//button[normalize-space(text())='Save topic']")),
      5000,
    )

    text = await bookmarksText(owner)
    assert(text.includes('No saved topics.'), 'unsaving should empty the saved list')

    console.log('e2e: bookmarks flow passed')
  } finally {
    await owner.quit()
    if (other) await other.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
