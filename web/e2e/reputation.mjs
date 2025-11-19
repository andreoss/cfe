import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { promoteViaRoot } from './support.mjs'

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

function kindButton(kind) {
  return By.xpath(`(//button[starts-with(normalize-space(.), '${kind} ')])[1]`)
}

async function register(driver, username) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 10000)
}

async function signIn(driver, username) {
  await driver.get(`${baseUrl}/sign-in`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 10000)
}

async function signOut(driver) {
  await driver.get(`${baseUrl}/`)
  await driver.wait(
    until.elementLocated(By.xpath("//button[normalize-space(.)='Sign out']")),
    5000,
  )
  await driver.findElement(By.xpath("//button[normalize-space(.)='Sign out']")).click()
  await driver.wait(
    async () => (await driver.findElement(By.css('nav')).getText()).includes('Sign in'),
    10000,
    'signing out should show the sign-in link',
  )
}

async function scoreOf(driver, username) {
  await driver.get(`${baseUrl}/u/${username}`)
  const element = await driver.wait(
    until.elementLocated(By.css('[data-test="score"]')),
    10000,
    'a profile should show a score',
  )
  return (await element.getText()).trim()
}

async function waitForScore(driver, username, expected) {
  let seen = ''
  for (let attempt = 0; attempt < 40; attempt += 1) {
    seen = await scoreOf(driver, username)
    if (seen === expected) return
    await new Promise((resolve) => setTimeout(resolve, 250))
  }
  throw new Error(`expected ${username} to show "${expected}", saw "${seen}"`)
}

async function reactWith(driver, topicUrl, kind) {
  await driver.get(topicUrl)
  const button = await driver.wait(
    until.elementLocated(kindButton(kind)),
    10000,
    `the ${kind} control should be on the topic page`,
  )
  await button.click()
}

async function run() {
  const driver = await buildDriver()
  const suffix = Date.now().toString(36)
  const author = `e2e_rep_a_${suffix}`
  const fan = `e2e_rep_f_${suffix}`
  try {
    await register(driver, author)
    await driver.get(`${baseUrl}/s/general`)
    await driver.wait(until.elementLocated(By.xpath("//button[text()='New topic']")), 10000)
    await driver.findElement(By.xpath("//button[text()='New topic']")).click()
    await driver.findElement(By.name('title')).sendKeys(`Reputation ${suffix}`)
    await driver.findElement(By.name('body')).sendKeys('A topic that will earn and lose.')
    await driver.findElement(By.xpath("//button[text()='Post']")).click()
    const link = await driver.wait(
      until.elementLocated(By.xpath(`//a[normalize-space(.)='Reputation ${suffix}']`)),
      10000,
      'the new topic should appear in the section',
    )
    await link.click()
    await driver.wait(until.elementLocated(By.name('comment-body')), 10000)
    const topicUrl = await driver.getCurrentUrl()

    assert(
      (await scoreOf(driver, author)) === 'Score: 0',
      'a new account should start at zero',
    )

    await reactWith(driver, topicUrl, 'thanks')
    assert(
      (await scoreOf(driver, author)) === 'Score: 0',
      'reacting to your own topic must not change your score',
    )

    await signOut(driver)
    await register(driver, fan)

    await reactWith(driver, topicUrl, 'like')
    await waitForScore(driver, author, 'Score: 1')

    await reactWith(driver, topicUrl, 'thanks')
    await waitForScore(driver, author, 'Score: 2')

    await reactWith(driver, topicUrl, 'disagree')
    await waitForScore(driver, author, 'Score: -1')

    await signOut(driver)
    assert(
      (await scoreOf(driver, author)) === 'Score: -1',
      'a signed-out visitor should see the score too',
    )

    await promoteViaRoot(fan)
    await signIn(driver, fan)
    await driver.get(topicUrl)
    await driver.wait(
      until.elementLocated(By.xpath("//button[text()='Delete topic']")),
      10000,
      'a moderator should see a delete control',
    )
    await driver.findElement(By.xpath("//button[text()='Delete topic']")).click()
    const reason = await driver.wait(until.elementLocated(By.name('delete-reason')), 5000)
    await reason.sendKeys('spam')
    await driver.findElement(By.xpath("//button[text()='Confirm delete']")).click()
    await driver.wait(until.elementLocated(By.css('.removed')), 10000)
    await waitForScore(driver, author, 'Score: -11')

    console.log('e2e: reputation flow passed')
  } finally {
    await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
