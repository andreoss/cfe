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

async function register(driver, username, invitation) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 10000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  if (invitation !== undefined) {
    await driver.wait(until.elementLocated(By.name('invitation')), 10000)
    await driver.findElement(By.name('invitation')).sendKeys(invitation)
  }
  await driver.findElement(By.css('button[type="submit"]')).click()
}

async function issueCode(driver) {
  await driver.get(`${baseUrl}/invitations`)
  const before = (await driver.findElements(By.css('.invitation-code'))).length
  await clickWhenReady(driver, By.xpath("//button[text()='Issue code']"), 'the issue control')
  await driver.wait(
    async () => (await driver.findElements(By.css('.invitation-code'))).length > before,
    10000,
    'issuing should add a code to the list',
  )
  const codes = await driver.findElements(By.css('.invitation-code'))
  return codes[0].getText()
}

async function run() {
  const host = await buildDriver()
  const guest = await buildDriver()
  const suffix = Date.now().toString(36)
  const hostName = `e2e_inv_h_${suffix}`
  const guestName = `e2e_inv_g_${suffix}`
  try {
    await register(host, hostName)
    await host.wait(until.urlIs(`${baseUrl}/`), 10000)

    await host.get(`${baseUrl}/invitations`)
    await host.wait(
      async () => (await mainText(host)).includes('You have not issued any invitations.'),
      10000,
      'the invitations list should start empty',
    )
    const empty = await mainText(host)
    assert(empty.includes('Invitations'), `the page should be headed Invitations, saw: ${empty}`)

    const code = await issueCode(host)
    assert(code.length === 12, `a code should be twelve characters, saw: ${code}`)

    const listed = await mainText(host)
    assert(listed.includes(code), `the issued code should be listed, saw: ${listed}`)
    assert(listed.includes('Unused'), `a fresh code should read as unused, saw: ${listed}`)
    assert(
      !listed.includes('You have not issued any invitations.'),
      'the empty message should be gone once a code exists',
    )

    await register(guest, guestName, code)
    await guest.wait(until.urlIs(`${baseUrl}/`), 10000)
    const guestNav = await guest.findElement(By.css('nav')).getText()
    assert(guestNav.includes(guestName), `registering with a code should sign you in: ${guestNav}`)

    await host.get(`${baseUrl}/invitations`)
    await host.wait(
      async () => (await mainText(host)).includes(`Used by ${guestName}`),
      10000,
      'the spent code should name who took it',
    )
    const spentText = await mainText(host)
    assert(spentText.includes(code), 'a spent code should still be listed')

    const repeat = await buildDriver()
    try {
      await register(repeat, `e2e_inv_r_${suffix}`, code)
      await repeat.wait(
        async () => (await mainText(repeat)).includes('invitation already used'),
        10000,
        'a code may only be spent once',
      )
      assert(
        (await repeat.getCurrentUrl()).includes('/register'),
        'a refused registration should stay on the register page',
      )
    } finally {
      await repeat.quit()
    }

    await host.get(`${baseUrl}/invitations`)
    await host.wait(until.elementLocated(By.xpath("//button[text()='Issue code']")), 10000)
    let refused = false
    for (let attempt = 0; attempt < 8 && !refused; attempt += 1) {
      const before = (await host.findElements(By.css('.invitation-code'))).length
      await clickWhenReady(host, By.xpath("//button[text()='Issue code']"), 'the issue control')
      await host.wait(
        async () => {
          const grown = (await host.findElements(By.css('.invitation-code'))).length > before
          return grown || (await mainText(host)).includes('Too many unused invitations.')
        },
        10000,
        'issuing should either list another code or say why it refused',
      )
      refused = (await mainText(host)).includes('Too many unused invitations.')
    }
    assert(refused, 'issuing past the outstanding cap should be refused')

    const stranger = await buildDriver()
    try {
      await stranger.get(`${baseUrl}/invitations`)
      await stranger.wait(until.elementLocated(By.css('main')), 10000)
      const strangerText = await mainText(stranger)
      assert(
        !strangerText.includes(code),
        `a signed-out visitor must not see somebody's codes, saw: ${strangerText}`,
      )
    } finally {
      await stranger.quit()
    }

    console.log('e2e: invitations flow passed')
  } finally {
    await host.quit()
    await guest.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
