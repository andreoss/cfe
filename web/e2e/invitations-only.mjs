import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'

const baseUrl = process.env.BASE_URL ?? 'http://127.0.0.1:58081'
const apiUrl = process.env.API_URL ?? 'http://127.0.0.1:58080'
const rootUser = process.env.E2E_ROOT_USER ?? 'e2e_root'
const rootPass = process.env.E2E_ROOT_PASS ?? 'correcthorse'
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
      return await driver.findElement(By.css('main')).getText()
    } catch (err) {
      last = err
      await new Promise((resolve) => setTimeout(resolve, 250))
    }
  }
  throw last
}

async function codeFromRoot() {
  const signIn = await fetch(`${apiUrl}/api/sign-in`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username: rootUser, password: rootPass }),
  })
  const cookie = signIn.headers.get('set-cookie')
  assert(cookie !== null, 'signing in as the first account should set a session cookie')
  const issued = await fetch(`${apiUrl}/api/invitations`, {
    method: 'POST',
    headers: { cookie: cookie.split(';')[0] },
  })
  assert(issued.status === 201, `issuing should succeed, got ${issued.status}`)
  const body = await issued.json()
  return body.code
}

async function fillRegister(driver, username, invitation) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 10000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  if (invitation !== undefined) {
    await driver.findElement(By.name('invitation')).sendKeys(invitation)
  }
  await driver.findElement(By.css('button[type="submit"]')).click()
}

async function run() {
  const driver = await buildDriver()
  const suffix = Date.now().toString(36)
  try {
    await driver.get(`${baseUrl}/register`)
    await driver.wait(
      async () => (await mainText(driver)).includes('An invitation code is required to register.'),
      10000,
      'a closed registration should say a code is required',
    )

    await fillRegister(driver, `e2e_closed_a_${suffix}`)
    await driver.wait(
      async () => (await mainText(driver)).includes('an invitation code is required'),
      10000,
      'registering with no code should be refused while registration is closed',
    )
    assert(
      (await driver.getCurrentUrl()).includes('/register'),
      'a refused registration should stay on the register page',
    )

    await fillRegister(driver, `e2e_closed_b_${suffix}`, 'ZZZZZZZZZZZZ')
    await driver.wait(
      async () => (await mainText(driver)).includes('unknown invitation code'),
      10000,
      'a code nobody issued should be refused by name',
    )

    const code = await codeFromRoot()
    const admitted = `e2e_closed_c_${suffix}`
    await fillRegister(driver, admitted, code)
    await driver.wait(until.urlIs(`${baseUrl}/`), 10000)
    const nav = await driver.findElement(By.css('nav')).getText()
    assert(nav.includes(admitted), `a valid code should let you in, nav was: ${nav}`)

    console.log('e2e: invitation-only registration flow passed')
  } finally {
    await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
