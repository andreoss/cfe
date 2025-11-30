import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { promoteViaRoot, publishTopic } from './support.mjs'

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

async function register(driver, username, email, password) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(email)
  await driver.findElement(By.name('password')).sendKeys(password)
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 5000)
}

async function run() {
  const suffix = Date.now().toString(36)
  const modUsername = `e2e_mod_${suffix}`
  const plainUsername = `e2e_plain_${suffix}`
  const email = (u) => `${u}@example.com`
  const password = 'correcthorse'
  const topicTitle = `Moderated topic ${suffix}`

  const modDriver = await buildDriver()
  const plainDriver = await buildDriver()
  try {
    await register(modDriver, modUsername, email(modUsername), password)
    await promoteViaRoot(modUsername)
    await register(plainDriver, plainUsername, email(plainUsername), password)

    await plainDriver.get(`${baseUrl}/s/general`)
    await plainDriver.wait(until.urlIs(`${baseUrl}/s/general`), 5000)
    await plainDriver.wait(
      until.elementLocated(By.xpath("//button[text()='New topic']")),
      10000,
      'the new topic control should appear in the section',
    )
    await plainDriver.wait(
      until.elementLocated(By.xpath("//button[text()='New topic']")),
      10000,
      'the new topic control should appear once posting is known to be allowed',
    )
    await plainDriver.findElement(By.xpath("//button[text()='New topic']")).click()
    await plainDriver.wait(until.elementLocated(By.name('title')), 10000)
    await plainDriver.findElement(By.name('title')).sendKeys(topicTitle)
    await plainDriver.findElement(By.name('body')).sendKeys('Please moderate me.')
    await plainDriver.findElement(By.xpath("//button[text()='Post']")).click()
    await publishTopic(topicTitle)
    await plainDriver.get(`${baseUrl}/s/general`)
    await plainDriver.wait(until.elementLocated(By.linkText(topicTitle)), 5000)
    await plainDriver.findElement(By.linkText(topicTitle)).click()
    await plainDriver.wait(until.elementLocated(By.css('.body')), 5000)
    const topicUrl = await plainDriver.getCurrentUrl()

    const plainDeleteButtons = await plainDriver.findElements(
      By.xpath("//button[text()='Delete topic']"),
    )
    assert(plainDeleteButtons.length === 0, 'a plain user must not see a delete control')

    await modDriver.get(topicUrl)
    await modDriver.wait(until.elementLocated(By.xpath("//button[text()='Delete topic']")), 8000)
    await modDriver.findElement(By.xpath("//button[text()='Delete topic']")).click()
    const reasonInput = await modDriver.wait(until.elementLocated(By.name('delete-reason')), 5000)
    await reasonInput.sendKeys('spam e2e test')
    await modDriver.findElement(By.xpath("//button[text()='Confirm delete']")).click()
    await modDriver.wait(until.elementLocated(By.css('.removed')), 5000)
    let bodyText = await modDriver.findElement(By.css('body')).getText()
    assert(bodyText.includes('spam e2e test'), 'moderator view should show the removal reason')

    await modDriver.get(`${baseUrl}/s/general`)
    await modDriver.wait(until.elementLocated(By.css('main')), 5000)
    const listedLinks = await modDriver.findElements(By.linkText(topicTitle))
    assert(listedLinks.length === 0, 'deleted topic must not appear in the section listing')

    await plainDriver.get(topicUrl)
    await plainDriver.wait(until.elementLocated(By.css('.removed')), 5000)
    bodyText = await plainDriver.findElement(By.css('body')).getText()
    assert(
      bodyText.includes('spam e2e test'),
      'a direct link should still show the removal reason to any visitor',
    )

    console.log('e2e: moderation flow passed')
  } finally {
    await modDriver.quit()
    await plainDriver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
