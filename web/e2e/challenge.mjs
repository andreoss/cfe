import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'

const baseUrl = process.env.BASE_URL ?? 'http://127.0.0.1:58081'
const chromedriverPath = process.env.CHROMEDRIVER_PATH
const secret = process.env.E2E_CHALLENGE_SECRET ?? 'open sesame'

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
      return await driver.findElement(By.css('main')).getText()
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

async function fillRegister(driver, username) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
}

async function openNewTopic(driver) {
  let last
  for (let attempt = 0; attempt < 10; attempt += 1) {
    try {
      await clickWhenReady(
        driver,
        By.xpath("//button[text()='New topic']"),
        'the new topic control should be present',
      )
      await driver.wait(until.elementLocated(By.name('title')), 2000)
      return
    } catch (err) {
      last = err
    }
  }
  throw last
}

async function run() {
  const driver = await buildDriver()
  const suffix = Date.now().toString(36)
  const username = `e2e_ch_${suffix}`
  const title = `Challenged topic ${suffix}`
  try {
    await fillRegister(driver, username)
    await driver.wait(
      async () => (await mainText(driver)).includes('Answer the challenge to continue.'),
      10000,
      'registering should ask for the challenge',
    )
    assert(
      (await driver.getCurrentUrl()).includes('/register'),
      'a challenged registration should stay on the register page',
    )

    const wrong = await driver.findElement(By.name('challenge-answer'))
    await wrong.clear()
    await wrong.sendKeys('not the phrase')
    await driver.findElement(By.css('button[type="submit"]')).click()
    await driver.wait(
      async () => (await driver.findElements(By.css('[role="alert"]'))).length > 0,
      10000,
      'a wrong answer should be reported',
    )
    assert(
      (await driver.getCurrentUrl()).includes('/register'),
      'a wrong answer must not register the account',
    )

    const right = await driver.findElement(By.name('challenge-answer'))
    await right.clear()
    await right.sendKeys(secret)
    await driver.findElement(By.css('button[type="submit"]')).click()
    await driver.wait(until.urlIs(`${baseUrl}/`), 10000, 'the right answer should register')

    await driver.get(`${baseUrl}/s/general`)
    await openNewTopic(driver)
    await driver.findElement(By.name('title')).sendKeys(title)
    await driver.findElement(By.name('body')).sendKeys('A body posted under a challenge.')
    await clickWhenReady(driver, By.xpath("//button[text()='Post']"), 'post control')
    await driver.wait(
      async () => (await mainText(driver)).includes('Answer the challenge to continue.'),
      10000,
      'posting should ask for the challenge',
    )

    const answer = await driver.findElement(By.name('topic-challenge-answer'))
    await answer.clear()
    await answer.sendKeys(secret)
    await clickWhenReady(driver, By.xpath("//button[text()='Post']"), 'post control')
    await driver.wait(
      async () => (await mainText(driver)).includes(title),
      10000,
      'the answered post should land',
    )

    console.log('e2e: challenge flow passed')
  } finally {
    await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
