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

async function sessionChecked(driver) {
  return driver.executeScript(
    'return performance.getEntriesByType("resource").some((entry) => entry.name.endsWith("/api/me"))',
  )
}

async function fillRegister(driver, username, password) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys(password)
  await driver.findElement(By.css('button[type="submit"]')).click()
}

async function signIn(driver, username, password) {
  await driver.get(`${baseUrl}/sign-in`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('password')).sendKeys(password)
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(async () => {
    if ((await driver.getCurrentUrl()) === `${baseUrl}/`) return true
    return (await driver.findElements(By.css('[role="alert"]'))).length > 0
  }, 10000, 'sign-in should either succeed or report an error')
  return driver.getCurrentUrl()
}

async function changePassword(driver, current, next) {
  await driver.get(`${baseUrl}/settings`)
  const currentInput = await driver.wait(
    until.elementLocated(By.name('current-password')),
    5000,
  )
  await currentInput.clear()
  await currentInput.sendKeys(current)
  const nextInput = await driver.findElement(By.name('new-password'))
  await nextInput.clear()
  await nextInput.sendKeys(next)
  await driver.findElement(By.xpath("//button[normalize-space(.)='Change password']")).click()
  await driver.wait(
    until.elementLocated(By.css('[role="status"], [role="alert"]')),
    10000,
    'the change-password form should report an outcome',
  )
  return driver.findElement(By.css('main')).getText()
}

async function run() {
  const driver = await buildDriver()
  const suffix = Date.now().toString(36)
  const username = `e2e_acct_${suffix}`
  const first = 'correcthorse'
  const second = 'brandnewpass'
  try {
    await fillRegister(driver, `${username}_s`, 'short')
    await driver.wait(
      until.elementLocated(By.css('[role="alert"]')),
      10000,
      'the register form should report why it refused the account',
    )
    let text = await driver.findElement(By.css('main')).getText()
    assert(
      text.toLowerCase().includes('password'),
      `a short password should be refused with a message, saw: ${text}`,
    )
    assert(
      (await driver.getCurrentUrl()).includes('/register'),
      'a refused registration should stay on the register page',
    )

    await fillRegister(driver, username, first)
    await driver.wait(until.urlIs(`${baseUrl}/`), 5000)

    text = await changePassword(driver, 'notitatall', second)
    assert(
      text.toLowerCase().includes('wrong current password'),
      `a wrong current password should be reported, saw: ${text}`,
    )

    text = await changePassword(driver, first, 'short')
    assert(
      text.toLowerCase().includes('invalid new password'),
      `a short new password should be refused, saw: ${text}`,
    )

    text = await changePassword(driver, first, second)
    assert(
      text.includes('Password changed.'),
      `changing the password should confirm, saw: ${text}`,
    )

    await driver.findElement(By.xpath("//button[normalize-space(.)='Sign out']")).click()
    await driver.wait(
      async () => (await driver.findElement(By.css('nav')).getText()).includes('Sign in'),
      10000,
      'signing out should put the sign-in link back in the nav',
    )

    await signIn(driver, username, first)
    text = await driver.findElement(By.css('main')).getText()
    assert(
      text.toLowerCase().includes('invalid credentials'),
      `the old password must stop working, saw: ${text}`,
    )

    await signIn(driver, username, second)
    await driver.wait(until.urlIs(`${baseUrl}/`), 5000)

    await driver.get(`${baseUrl}/settings`)
    await driver.wait(
      until.elementLocated(By.xpath("//button[normalize-space(.)='Deregister account']")),
      5000,
    )
    await driver.findElement(By.xpath("//button[normalize-space(.)='Deregister account']")).click()
    const confirm = await driver.wait(
      until.elementLocated(By.xpath("//button[normalize-space(.)='Confirm deregister']")),
      5000,
    )
    await confirm.click()
    await driver.wait(
      async () => (await driver.findElement(By.css('nav')).getText()).includes('Sign in'),
      10000,
      'deregistering should sign you out',
    )

    const nav = await driver.findElement(By.css('nav')).getText()
    assert(nav.includes('Sign in'), `deregistering should sign you out, nav was: ${nav}`)

    await signIn(driver, username, second)
    text = await driver.findElement(By.css('main')).getText()
    assert(
      text.toLowerCase().includes('invalid credentials'),
      `a deregistered account must not sign in, saw: ${text}`,
    )

    await driver.get(`${baseUrl}/settings`)
    await driver.wait(until.elementLocated(By.css('main')), 5000)
    await driver.wait(
      async () => sessionChecked(driver),
      10000,
      'the settings page should have finished checking whose session this is',
    )
    text = await driver.findElement(By.css('main')).getText()
    assert(
      text.includes('Sign in to manage your account.'),
      `settings should be gated when signed out, saw: ${text}`,
    )

    console.log('e2e: account flow passed')
  } finally {
    await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
