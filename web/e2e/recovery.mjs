import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { readFileSync } from 'node:fs'

const baseUrl = process.env.BASE_URL ?? 'http://127.0.0.1:58081'
const mailLog = process.env.MAIL_LOG ?? '/tmp/tcbs-e2e-mail.log'
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

function messages() {
  let raw = ''
  try {
    raw = readFileSync(mailLog, 'utf8')
  } catch {
    return []
  }
  return raw
    .split('---\n')
    .filter((block) => block.includes('to='))
    .map((block) => ({
      to: (block.match(/^to=(.*)$/m) ?? [])[1],
      code: (block.match(/[0-9a-f]{64}/) ?? [])[0],
    }))
}

function countFor(address) {
  return messages().filter((m) => m.to === address).length
}

async function waitForCode(address, previous) {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    const mine = messages().filter((m) => m.to === address)
    if (mine.length > previous) {
      const code = mine[mine.length - 1].code
      if (code) return code
    }
    await new Promise((resolve) => setTimeout(resolve, 200))
  }
  throw new Error(`no new mail reached ${address}`)
}

async function outcome(driver) {
  await driver.wait(
    until.elementLocated(By.css('[role="status"], [role="alert"]')),
    10000,
    'the form should report an outcome',
  )
  return driver.findElement(By.css('main')).getText()
}

async function requestReset(driver, address) {
  await driver.get(`${baseUrl}/forgot-password`)
  const input = await driver.wait(until.elementLocated(By.name('reset-email')), 5000)
  await input.clear()
  await input.sendKeys(address)
  await driver.findElement(By.xpath("//button[normalize-space(.)='Send reset code']")).click()
  return outcome(driver)
}

async function redeem(driver, code, password) {
  const codeInput = await driver.wait(until.elementLocated(By.name('reset-code')), 5000)
  await codeInput.clear()
  await codeInput.sendKeys(code)
  const passwordInput = await driver.findElement(By.name('reset-password'))
  await passwordInput.clear()
  await passwordInput.sendKeys(password)
  await driver.findElement(By.xpath("//button[normalize-space(.)='Set new password']")).click()
  const message = By.xpath(
    "//form[.//button[normalize-space(.)='Set new password']]" +
      "//*[@role='status' or @role='alert']",
  )
  await driver.wait(
    until.elementLocated(message),
    10000,
    'the redeem form should report its own outcome',
  )
  return driver.findElement(message).getText()
}

async function signIn(driver, username, password) {
  await driver.get(`${baseUrl}/sign-in`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('password')).sendKeys(password)
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(
    async () => {
      if ((await driver.getCurrentUrl()) === `${baseUrl}/`) return true
      return (await driver.findElements(By.css('[role="alert"]'))).length > 0
    },
    10000,
    'sign-in should either succeed or report an error',
  )
}

async function run() {
  const driver = await buildDriver()
  const suffix = Date.now().toString(36)
  const username = `e2e_lost_${suffix}`
  const address = `${username}@example.com`
  const first = 'correcthorse'
  const second = 'brandnewpass'
  try {
    const beforeRegister = countFor(address)
    await driver.get(`${baseUrl}/register`)
    await driver.wait(until.elementLocated(By.name('username')), 5000)
    await driver.findElement(By.name('username')).sendKeys(username)
    await driver.findElement(By.name('email')).sendKeys(address)
    await driver.findElement(By.name('password')).sendKeys(first)
    await driver.findElement(By.css('button[type="submit"]')).click()
    await driver.wait(until.urlIs(`${baseUrl}/`), 10000)

    const activation = await waitForCode(address, beforeRegister)
    await driver.get(`${baseUrl}/activate`)
    const activationInput = await driver.wait(
      until.elementLocated(By.name('activation-code')),
      5000,
    )
    await activationInput.sendKeys(activation)
    await driver.findElement(By.xpath("//button[normalize-space(.)='Confirm address']")).click()
    let text = await outcome(driver)
    assert(text.includes('Address confirmed.'), `activation should confirm, saw: ${text}`)

    await driver.get(`${baseUrl}/sign-in`)
    await driver.wait(until.elementLocated(By.name('username')), 5000)
    await driver
      .findElement(By.xpath("//a[normalize-space(.)='Forgot password?']"))
      .click()
    await driver.wait(until.urlIs(`${baseUrl}/forgot-password`), 5000)

    const unknown = `nobody_${suffix}@example.com`
    const beforeUnknown = messages().length
    const unknownText = await requestReset(driver, unknown)
    assert(
      unknownText.includes('If that address has an account, a reset code is on its way.'),
      `an unknown address must get the neutral message, saw: ${unknownText}`,
    )
    await new Promise((resolve) => setTimeout(resolve, 1000))
    assert(
      messages().length === beforeUnknown,
      'an unknown address must not cause any mail to be sent',
    )

    const beforeReset = countFor(address)
    const knownText = await requestReset(driver, address)
    assert(
      knownText.includes('If that address has an account, a reset code is on its way.'),
      `a known address must get the same neutral message, saw: ${knownText}`,
    )
    const code = await waitForCode(address, beforeReset)

    text = await redeem(driver, 'deadbeef'.repeat(8), second)
    assert(
      text.toLowerCase().includes('invalid or expired code'),
      `a wrong code should be refused, saw: ${text}`,
    )

    text = await redeem(driver, code, 'short')
    assert(
      text.toLowerCase().includes('invalid new password'),
      `a short new password should be refused, saw: ${text}`,
    )

    text = await redeem(driver, code, second)
    assert(
      text.includes('Password changed. You can sign in now.'),
      `the mailed code should set the new password, saw: ${text}`,
    )

    await signIn(driver, username, first)
    text = await driver.findElement(By.css('main')).getText()
    assert(
      text.toLowerCase().includes('invalid credentials'),
      `the forgotten password must stop working, saw: ${text}`,
    )

    await signIn(driver, username, second)
    await driver.wait(until.urlIs(`${baseUrl}/`), 5000)

    await driver.get(`${baseUrl}/forgot-password`)
    text = await redeem(driver, code, 'thirdpassword')
    assert(
      text.toLowerCase().includes('invalid or expired code'),
      `a redeemed code must not work twice, saw: ${text}`,
    )

    console.log('e2e: recovery flow passed')
  } finally {
    await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
